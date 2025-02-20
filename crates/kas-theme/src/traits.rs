// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Theme traits

use kas::draw::{color, DrawIface, DrawSharedImpl, SharedState};
use kas::event::EventState;
use kas::theme::{DrawHandle, SizeHandle, ThemeControl};
use kas::TkAction;
use std::any::Any;
use std::ops::{Deref, DerefMut};

/// Requirements on theme config (without `config` feature)
#[cfg(not(feature = "config"))]
pub trait ThemeConfig: Clone + std::fmt::Debug + 'static {
    /// Apply startup effects
    fn apply_startup(&self);

    /// Get raster config
    fn raster(&self) -> &crate::RasterConfig;
}

/// Requirements on theme config (with `config` feature)
#[cfg(feature = "config")]
pub trait ThemeConfig:
    Clone + std::fmt::Debug + 'static + for<'a> serde::Deserialize<'a> + serde::Serialize
{
    /// Has the config ever been updated?
    fn is_dirty(&self) -> bool;

    /// Apply startup effects
    fn apply_startup(&self);

    /// Get raster config
    fn raster(&self) -> &crate::RasterConfig;
}

/// A *theme* provides widget sizing and drawing implementations.
///
/// The theme is generic over some `DrawIface`.
///
/// Objects of this type are copied within each window's data structure. For
/// large resources (e.g. fonts and icons) consider using external storage.
pub trait Theme<DS: DrawSharedImpl>: ThemeControl {
    /// The associated config type
    type Config: ThemeConfig;

    /// The associated [`Window`] implementation.
    type Window: Window;

    /// The associated [`DrawHandle`] implementation.
    #[cfg(not(feature = "gat"))]
    type DrawHandle: DrawHandle;
    #[cfg(feature = "gat")]
    type DrawHandle<'a>: DrawHandle
    where
        DS: 'a,
        Self: 'a;

    /// Get current config
    fn config(&self) -> std::borrow::Cow<Self::Config>;

    /// Apply/set the passed config
    fn apply_config(&mut self, config: &Self::Config) -> TkAction;

    /// Theme initialisation
    ///
    /// The toolkit must call this method before [`Theme::new_window`]
    /// to allow initialisation specific to the `DrawIface`.
    ///
    /// At a minimum, a theme must load a font to [`kas::text::fonts`].
    /// The first font loaded (by any theme) becomes the default font.
    fn init(&mut self, shared: &mut SharedState<DS>);

    /// Construct per-window storage
    ///
    /// On "standard" monitors, the `dpi_factor` is 1. High-DPI screens may
    /// have a factor of 2 or higher. The factor may not be an integer; e.g.
    /// `9/8 = 1.125` works well with many 1440p screens. It is recommended to
    /// round dimensions to the nearest integer, and cache the result:
    /// ```notest
    /// self.margin = i32::conv_nearest(MARGIN * factor);
    /// ```
    ///
    /// A reference to the draw backend is provided allowing configuration.
    fn new_window(&self, dpi_factor: f32) -> Self::Window;

    /// Update a window created by [`Theme::new_window`]
    ///
    /// This is called when the DPI factor changes or theme dimensions change.
    fn update_window(&self, window: &mut Self::Window, dpi_factor: f32);

    /// Prepare to draw and construct a [`DrawHandle`] object
    ///
    /// This is called once per window per frame and should do any necessary
    /// preparation such as loading fonts and textures which are loaded on
    /// demand.
    ///
    /// Drawing via this [`DrawHandle`] is restricted to the specified `rect`.
    ///
    /// The `window` is guaranteed to be one created by a call to
    /// [`Theme::new_window`] on `self`, and the `draw` reference is guaranteed
    /// to be identical to the one passed to [`Theme::new_window`].
    ///
    /// # Safety
    ///
    /// (This section only applies when not using the `gat` feature.)
    ///
    /// All references passed into the method must outlive the returned object.
    #[cfg(not(feature = "gat"))]
    unsafe fn draw_handle(
        &self,
        draw: DrawIface<DS>,
        ev: &mut EventState,
        window: &mut Self::Window,
    ) -> Self::DrawHandle;
    #[cfg(feature = "gat")]
    fn draw_handle<'a>(
        &'a self,
        draw: DrawIface<'a, DS>,
        ev: &'a mut EventState,
        window: &'a mut Self::Window,
    ) -> Self::DrawHandle<'a>;

    /// Background colour
    fn clear_color(&self) -> color::Rgba;
}

/// Per-window storage for the theme
///
/// Constructed via [`Theme::new_window`].
///
/// The main reason for this separation is to allow proper handling of
/// multi-window applications across screens with differing DPIs.
pub trait Window: 'static {
    /// Construct a [`SizeHandle`] object
    fn size_handle(&self) -> &dyn SizeHandle;

    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<T: Theme<DS>, DS: DrawSharedImpl> Theme<DS> for Box<T> {
    type Window = <T as Theme<DS>>::Window;
    type Config = <T as Theme<DS>>::Config;

    #[cfg(not(feature = "gat"))]
    type DrawHandle = <T as Theme<DS>>::DrawHandle;
    #[cfg(feature = "gat")]
    type DrawHandle<'a>
    where
        T: 'a,
    = <T as Theme<DS>>::DrawHandle<'a>;

    fn config(&self) -> std::borrow::Cow<Self::Config> {
        self.deref().config()
    }
    fn apply_config(&mut self, config: &Self::Config) -> TkAction {
        self.deref_mut().apply_config(config)
    }

    fn init(&mut self, shared: &mut SharedState<DS>) {
        self.deref_mut().init(shared);
    }

    fn new_window(&self, dpi_factor: f32) -> Self::Window {
        self.deref().new_window(dpi_factor)
    }
    fn update_window(&self, window: &mut Self::Window, dpi_factor: f32) {
        self.deref().update_window(window, dpi_factor);
    }

    #[cfg(not(feature = "gat"))]
    unsafe fn draw_handle(
        &self,
        draw: DrawIface<DS>,
        ev: &mut EventState,
        window: &mut Self::Window,
    ) -> Self::DrawHandle {
        self.deref().draw_handle(draw, ev, window)
    }
    #[cfg(feature = "gat")]
    fn draw_handle<'a>(
        &'a self,
        draw: DrawIface<'a, DS>,
        ev: &'a mut EventState,
        window: &'a mut Self::Window,
    ) -> Self::DrawHandle<'a> {
        self.deref().draw_handle(draw, ev, window)
    }

    fn clear_color(&self) -> color::Rgba {
        self.deref().clear_color()
    }
}

impl<W: Window> Window for Box<W> {
    fn size_handle(&self) -> &dyn SizeHandle {
        self.deref().size_handle()
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self.deref_mut().as_any_mut()
    }
}
