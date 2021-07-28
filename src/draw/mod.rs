// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! # Draw APIs
//!
//! Multiple drawing APIs are available. Each has a slightly different purpose.
//!
//! ### High-level themeable interface
//!
//! When widgets are sized or drawn, they are provided a [`SizeHandle`] or a
//! [`DrawHandle`] trait object. A [`SizeHandle`] may also be obtained through
//! [`kas::event::Manager::size_handle`].
//!
//! These traits are implemented by the theme of choice, providing a high-level
//! themed API over "widget features".
//!
//! [`SizeHandle`] is the only part of the API providing sizing data. If drawing
//! via a lower-level API, it may still be necessary to query the scale factor
//! or some feature size via [`SizeHandle`].
//!
//! ### Medium-level drawing interfaces
//!
//! The theme draws widget components over a [`DrawIface`] object.
//! Widgets may access this same API via [`DrawHandle::draw_device`].
//!
//! The traits [`DrawT`] and [`DrawRoundedT`] provide functinality over a
//! [`DrawIface`] object. Additional interfaces may be defined in external crates.
//!
//! ### Low-level interface
//!
//! There is no universal graphics API, hence none is provided by this crate.
//! Instead, shells may provide their own extensions allowing direct access
//! to the host graphics API, for example
//! [`kas-wgpu::draw::CustomPipe`](https://docs.rs/kas-wgpu/*/kas_wgpu/draw/trait.CustomPipe.html).
//! The `mandlebrot` example demonstrates use of a custom draw pipe.
//!
//! ## Draw order
//!
//! All draw operations happen within a "draw pass". The first pass corresponds
//! to the window, while additional passes may be clipped and offset (see
//! [`DrawIface::new_draw_pass`]). Draw passes are executed sequentially in the order
//! defined.
//!
//! Within each pass, draw operations may be batched by the shell, thus draw
//! operations may not happen in the order queued. In general, it may be
//! expected that batches are executed in the following order:
//!
//! 1.  Images
//! 2.  Square-edged primitives (e.g. [`Draw::rect`])
//! 3.  Rounded or other partially-transparent primitives (e.g. [`Draw::circle`])
//! 4.  Custom draw routines (`CustomPipe`)
//! 5.  Text

pub mod color;

#[allow(clippy::module_inception)]
mod draw;
mod draw_shared;
mod handle;
mod images;
mod theme;

use crate::cast::Cast;

pub use draw::*;
pub use draw_shared::{SharedState, DrawSharedT, DrawSharedImpl};
pub use handle::*;
pub use images::{ImageError, ImageFormat, ImageId};
pub use theme::*;

/// Draw pass identifier
///
/// This is a small value passed by copy to identify which pass draw routines
/// happen in. Draw passes allow control over draw order and clipping.
///
/// Custom render pipes should extract the pass number.
#[derive(Copy, Clone)]
pub struct PassId(u32);

impl PassId {
    /// Construct a new pass from a `u32` identifier
    #[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
    #[cfg_attr(doc_cfg, doc(cfg(internal_doc)))]
    #[inline]
    pub const fn new(n: u32) -> Self {
        PassId(n)
    }

    /// The pass number
    ///
    /// This value is returned as `usize` but is always safe to store `as u32`.
    #[inline]
    pub fn pass(self) -> usize {
        self.0.cast()
    }

    /// The depth value
    ///
    /// This is a historical left-over and always returns 0.0.
    #[inline]
    pub fn depth(self) -> f32 {
        0.0
    }
}
