// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Simple pipeline for "square" shading

use std::f32;
use std::mem::size_of;
use wgpu::util::DeviceExt;

use crate::draw::{Rgb, ShaderManager};
use kas::conv::Conv;
use kas::draw::{Colour, Pass};
use kas::geom::{Quad, Size, Vec2, Vec3};

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct Vertex(Vec3, Rgb, Vec2);
unsafe impl bytemuck::Zeroable for Vertex {}
unsafe impl bytemuck::Pod for Vertex {}

/// A pipeline for rendering with flat and square-corner shading
pub struct Pipeline {
    bind_group_layout: wgpu::BindGroupLayout,
    render_pipeline: wgpu::RenderPipeline,
}

/// Per-window state
pub struct Window {
    bind_group: wgpu::BindGroup,
    scale_buf: wgpu::Buffer,
    passes: Vec<Vec<Vertex>>,
}

/// Buffer used during render pass
///
/// This buffer must not be dropped before the render pass.
pub struct RenderBuffer<'a> {
    pipe: &'a wgpu::RenderPipeline,
    vertices: &'a mut Vec<Vertex>,
    bind_group: &'a wgpu::BindGroup,
    buffer: wgpu::Buffer,
}

impl<'a> RenderBuffer<'a> {
    /// Do the render
    pub fn render(&'a self, rpass: &mut wgpu::RenderPass<'a>) {
        let count = u32::conv(self.vertices.len());
        rpass.set_pipeline(self.pipe);
        rpass.set_bind_group(0, self.bind_group, &[]);
        rpass.set_vertex_buffer(0, self.buffer.slice(..));
        rpass.draw(0..count, 0..1);
    }
}

impl<'a> Drop for RenderBuffer<'a> {
    fn drop(&mut self) {
        self.vertices.clear();
    }
}

impl Pipeline {
    /// Construct
    pub fn new(device: &wgpu::Device, shaders: &ShaderManager) -> Self {
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("SS bind_group_layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStage::VERTEX,
                    ty: wgpu::BindingType::UniformBuffer {
                        dynamic: false,
                        min_binding_size: None, // TODO
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::UniformBuffer {
                        dynamic: false,
                        min_binding_size: None, // TODO
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("SS pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("SS render_pipeline"),
            layout: Some(&pipeline_layout),
            vertex_stage: wgpu::ProgrammableStageDescriptor {
                module: &shaders.vert_32,
                entry_point: "main",
            },
            fragment_stage: Some(wgpu::ProgrammableStageDescriptor {
                module: &shaders.frag_shaded_square,
                entry_point: "main",
            }),
            rasterization_state: Some(wgpu::RasterizationStateDescriptor {
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: wgpu::CullMode::None,
                clamp_depth: false,
                depth_bias: 0,
                depth_bias_slope_scale: 0.0,
                depth_bias_clamp: 0.0,
            }),
            primitive_topology: wgpu::PrimitiveTopology::TriangleList,
            color_states: &[wgpu::ColorStateDescriptor {
                format: wgpu::TextureFormat::Bgra8UnormSrgb,
                color_blend: wgpu::BlendDescriptor::REPLACE,
                alpha_blend: wgpu::BlendDescriptor::REPLACE,
                write_mask: wgpu::ColorWrite::ALL,
            }],
            depth_stencil_state: Some(super::DEPTH_DESC),
            vertex_state: wgpu::VertexStateDescriptor {
                index_format: wgpu::IndexFormat::Uint16,
                vertex_buffers: &[wgpu::VertexBufferDescriptor {
                    stride: size_of::<Vertex>() as wgpu::BufferAddress,
                    step_mode: wgpu::InputStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float3, 1 => Float3, 2 => Float2],
                }],
            },
            sample_count: 1,
            sample_mask: !0,
            alpha_to_coverage_enabled: false,
        });

        Pipeline {
            bind_group_layout,
            render_pipeline,
        }
    }

    /// Construct per-window state
    pub fn new_window(&self, device: &wgpu::Device, size: Size, light_norm: [f32; 3]) -> Window {
        type Scale = [f32; 2];
        let scale_factor: Scale = [2.0 / size.0 as f32, -2.0 / size.1 as f32];
        let scale_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("SS scale_buf"),
            contents: bytemuck::cast_slice(&scale_factor),
            usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
        });

        let light_norm_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("SS light_norm_buf"),
            contents: bytemuck::cast_slice(&light_norm),
            usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("SS bind group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(scale_buf.slice(..)),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Buffer(light_norm_buf.slice(..)),
                },
            ],
        });

        Window {
            bind_group,
            scale_buf,
            passes: vec![],
        }
    }

    /// Construct a render buffer
    pub fn render_buf<'a>(
        &'a self,
        window: &'a mut Window,
        device: &wgpu::Device,
        pass: usize,
    ) -> Option<RenderBuffer<'a>> {
        if pass >= window.passes.len() || window.passes[pass].len() == 0 {
            return None;
        }

        let vertices = &mut window.passes[pass];
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("SS render_buf"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsage::VERTEX,
        });

        Some(RenderBuffer {
            pipe: &self.render_pipeline,
            vertices,
            bind_group: &window.bind_group,
            buffer,
        })
    }
}

impl Window {
    pub fn resize(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        size: Size,
    ) {
        type Scale = [f32; 2];
        let scale_factor: Scale = [2.0 / size.0 as f32, -2.0 / size.1 as f32];
        let scale_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("SS scale_buf copy"),
            contents: bytemuck::cast_slice(&scale_factor),
            usage: wgpu::BufferUsage::COPY_SRC,
        });
        let byte_len = u64::conv(size_of::<Scale>());

        encoder.copy_buffer_to_buffer(&scale_buf, 0, &self.scale_buf, 0, byte_len);
    }

    /// Add a rectangle to the buffer
    pub fn rect(&mut self, pass: Pass, rect: Quad, col: Colour) {
        let aa = rect.a;
        let bb = rect.b;

        if !aa.lt(bb) {
            // zero / negative size: nothing to draw
            return;
        }

        let depth = pass.depth();
        let ab = Vec3(aa.0, bb.1, depth);
        let ba = Vec3(bb.0, aa.1, depth);
        let aa = Vec3::from2(aa, depth);
        let bb = Vec3::from2(bb, depth);

        let col = col.into();
        let t = Vec2(0.0, 0.0);

        #[rustfmt::skip]
        self.add_vertices(pass.pass(), &[
            Vertex(aa, col, t), Vertex(ba, col, t), Vertex(ab, col, t),
            Vertex(ab, col, t), Vertex(ba, col, t), Vertex(bb, col, t),
        ]);
    }

    /// Add a rect to the buffer, defined by two outer corners, `aa` and `bb`.
    ///
    /// Bounds on input: `aa < cc` and `-1 ≤ norm ≤ 1`.
    pub fn shaded_rect(&mut self, pass: Pass, rect: Quad, mut norm: Vec2, col: Colour) {
        let aa = rect.a;
        let bb = rect.b;

        if !aa.lt(bb) {
            // zero / negative size: nothing to draw
            return;
        }
        if !Vec2::splat(-1.0).le(norm) || !norm.le(Vec2::splat(1.0)) {
            norm = Vec2::splat(0.0);
        }

        let depth = pass.depth();
        let mid = Vec3::from2((aa + bb) * 0.5, depth);
        let ab = Vec3(aa.0, bb.1, depth);
        let ba = Vec3(bb.0, aa.1, depth);
        let aa = Vec3::from2(aa, depth);
        let bb = Vec3::from2(bb, depth);

        let col = col.into();
        let tt = (Vec2(0.0, -norm.1), Vec2(0.0, -norm.0));
        let tl = (Vec2(-norm.1, 0.0), Vec2(-norm.0, 0.0));
        let tb = (Vec2(0.0, norm.1), Vec2(0.0, norm.0));
        let tr = (Vec2(norm.1, 0.0), Vec2(norm.0, 0.0));

        #[rustfmt::skip]
        self.add_vertices(pass.pass(), &[
            Vertex(ba, col, tt.0), Vertex(mid, col, tt.1), Vertex(aa, col, tt.0),
            Vertex(aa, col, tl.0), Vertex(mid, col, tl.1), Vertex(ab, col, tl.0),
            Vertex(ab, col, tb.0), Vertex(mid, col, tb.1), Vertex(bb, col, tb.0),
            Vertex(bb, col, tr.0), Vertex(mid, col, tr.1), Vertex(ba, col, tr.0),
        ]);
    }

    #[inline]
    pub fn frame(&mut self, pass: Pass, outer: Quad, inner: Quad, col: Colour) {
        let norm = Vec2::splat(0.0);
        self.shaded_frame(pass, outer, inner, norm, col);
    }

    /// Add a frame to the buffer, defined by two outer corners, `aa` and `bb`,
    /// and two inner corners, `cc` and `dd` with colour `col`.
    ///
    /// Bounds on input: `aa < cc < dd < bb` and `-1 ≤ norm ≤ 1`.
    pub fn shaded_frame(
        &mut self,
        pass: Pass,
        outer: Quad,
        inner: Quad,
        mut norm: Vec2,
        col: Colour,
    ) {
        let aa = outer.a;
        let bb = outer.b;
        let mut cc = inner.a;
        let mut dd = inner.b;

        if !aa.lt(bb) {
            // zero / negative size: nothing to draw
            return;
        }
        if !aa.le(cc) || !cc.le(bb) {
            cc = aa;
        }
        if !aa.le(dd) || !dd.le(bb) {
            dd = bb;
        }
        if !cc.le(dd) {
            dd = cc;
        }
        if !Vec2::splat(-1.0).le(norm) || !norm.le(Vec2::splat(1.0)) {
            norm = Vec2::splat(0.0);
        }

        let depth = pass.depth();
        let ab = Vec3(aa.0, bb.1, depth);
        let ba = Vec3(bb.0, aa.1, depth);
        let cd = Vec3(cc.0, dd.1, depth);
        let dc = Vec3(dd.0, cc.1, depth);
        let aa = Vec3::from2(aa, depth);
        let bb = Vec3::from2(bb, depth);
        let cc = Vec3::from2(cc, depth);
        let dd = Vec3::from2(dd, depth);

        let col = col.into();
        let tt = (Vec2(0.0, -norm.1), Vec2(0.0, -norm.0));
        let tl = (Vec2(-norm.1, 0.0), Vec2(-norm.0, 0.0));
        let tb = (Vec2(0.0, norm.1), Vec2(0.0, norm.0));
        let tr = (Vec2(norm.1, 0.0), Vec2(norm.0, 0.0));

        #[rustfmt::skip]
        self.add_vertices(pass.pass(), &[
            // top bar: ba - dc - cc - aa
            Vertex(ba, col, tt.0), Vertex(dc, col, tt.1), Vertex(aa, col, tt.0),
            Vertex(aa, col, tt.0), Vertex(dc, col, tt.1), Vertex(cc, col, tt.1),
            // left bar: aa - cc - cd - ab
            Vertex(aa, col, tl.0), Vertex(cc, col, tl.1), Vertex(ab, col, tl.0),
            Vertex(ab, col, tl.0), Vertex(cc, col, tl.1), Vertex(cd, col, tl.1),
            // bottom bar: ab - cd - dd - bb
            Vertex(ab, col, tb.0), Vertex(cd, col, tb.1), Vertex(bb, col, tb.0),
            Vertex(bb, col, tb.0), Vertex(cd, col, tb.1), Vertex(dd, col, tb.1),
            // right bar: bb - dd - dc - ba
            Vertex(bb, col, tr.0), Vertex(dd, col, tr.1), Vertex(ba, col, tr.0),
            Vertex(ba, col, tr.0), Vertex(dd, col, tr.1), Vertex(dc, col, tr.1),
        ]);
    }

    fn add_vertices(&mut self, pass: usize, slice: &[Vertex]) {
        if self.passes.len() <= pass {
            // We only need one more, but no harm in adding extra
            self.passes.resize(pass + 8, vec![]);
        }

        self.passes[pass].extend_from_slice(slice);
    }
}
