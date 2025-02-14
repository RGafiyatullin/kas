// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! "Handle" types used by themes

use std::ops::Deref;

use super::{FrameStyle, MarkStyle, TextClass};
use crate::dir::Directional;
use crate::geom::{Size, Vec2};
use crate::layout::{AxisInfo, FrameRules, Margins, SizeRules};
use crate::macros::autoimpl;
use crate::text::{Align, TextApi};
#[allow(unused)]
use crate::{layout::SetRectMgr, theme::DrawMgr};

// for doc use
#[allow(unused)]
use crate::text::TextApiExt;

/// Size and scale interface
///
/// This interface is provided to widgets in [`crate::Layout::size_rules`].
/// It may also be accessed through [`crate::event::EventMgr::size_mgr`],
/// [`DrawMgr::size_mgr`].
///
/// Most methods get or calculate the size of some feature. These same features
/// may be drawn through [`DrawMgr`].
pub struct SizeMgr<'a>(&'a dyn SizeHandle);

impl<'a> SizeMgr<'a> {
    /// Construct from a [`SizeHandle`]
    #[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
    #[cfg_attr(doc_cfg, doc(cfg(internal_doc)))]
    pub fn new(h: &'a dyn SizeHandle) -> Self {
        SizeMgr(h)
    }

    /// Reborrow with a new lifetime
    ///
    /// Rust allows references like `&T` or `&mut T` to be "reborrowed" through
    /// coercion: essentially, the pointer is copied under a new, shorter, lifetime.
    /// Until rfcs#1403 lands, reborrows on user types require a method call.
    ///
    /// Calling this method is zero-cost.
    #[inline(always)]
    pub fn re<'b>(&'b self) -> SizeMgr<'b>
    where
        'a: 'b,
    {
        SizeMgr(self.0)
    }

    /// Get the scale (DPI) factor
    ///
    /// "Traditional" PC screens have a scale factor of 1; high-DPI screens
    /// may have a factor of 2 or higher; this may be fractional. It is
    /// recommended to calculate sizes as follows:
    /// ```
    /// use kas_core::cast::*;
    /// # let scale_factor = 1.5f32;
    /// let size: i32 = (100.0 * scale_factor).cast_ceil();
    /// ```
    ///
    /// This value may change during a program's execution (e.g. when a window
    /// is moved to a different monitor); in this case all widgets will be
    /// resized via [`crate::Layout::size_rules`].
    pub fn scale_factor(&self) -> f32 {
        self.0.scale_factor()
    }

    /// Convert a size in virtual pixels to physical pixels
    pub fn pixels_from_virtual(&self, px: f32) -> f32 {
        px * self.scale_factor()
    }

    /// Convert a size in font Points to physical pixels
    pub fn pixels_from_points(&self, pt: f32) -> f32 {
        self.0.pixels_from_points(pt)
    }

    /// Convert a size in font Em to physical pixels
    ///
    /// (This depends on the font size.)
    pub fn pixels_from_em(&self, em: f32) -> f32 {
        self.0.pixels_from_em(em)
    }

    /// Size of a frame around another element
    pub fn frame(&self, style: FrameStyle, dir: impl Directional) -> FrameRules {
        self.0.frame(style, dir.is_vertical())
    }

    /// Size of a separator frame between items
    pub fn separator(&self) -> Size {
        self.0.separator()
    }

    /// The margin around content within a widget
    ///
    /// Though inner margins are *usually* empty, they are sometimes drawn to,
    /// for example focus indicators.
    pub fn inner_margin(&self) -> Size {
        self.0.inner_margin()
    }

    /// The margin between UI elements, where desired
    ///
    /// Widgets must not draw in outer margins.
    pub fn outer_margins(&self) -> Margins {
        self.0.outer_margins()
    }

    /// The margin around text elements
    ///
    /// Similar to [`Self::outer_margins`], but intended for things like text
    /// labels which do not have a visible hard edge.
    pub fn text_margins(&self) -> Margins {
        self.0.text_margins()
    }

    /// The height of a line of text
    pub fn line_height(&self, class: TextClass) -> i32 {
        self.0.line_height(class)
    }

    /// Update a text object, setting font properties and getting a size bound
    ///
    /// This method updates the text's [`Environment`] and uses the result to
    /// calculate size requirements.
    ///
    /// It is necessary to update the environment *again* once the target `rect`
    /// is known: use [`SetRectMgr::text_set_size`] to do this.
    ///
    /// [`Environment`]: crate::text::Environment
    pub fn text_bound(
        &self,
        text: &mut dyn TextApi,
        class: TextClass,
        axis: AxisInfo,
    ) -> SizeRules {
        self.0.text_bound(text, class, axis)
    }

    /// Size of the element drawn by [`DrawMgr::checkbox`].
    pub fn checkbox(&self) -> Size {
        self.0.checkbox()
    }

    /// Size of the element drawn by [`DrawMgr::radiobox`].
    pub fn radiobox(&self) -> Size {
        self.0.radiobox()
    }

    /// A simple mark
    pub fn mark(&self, style: MarkStyle, dir: impl Directional) -> SizeRules {
        self.0.mark(style, dir.is_vertical())
    }

    /// Dimensions for a scrollbar
    ///
    /// Returns:
    ///
    /// -   `size`: minimum size of handle in horizontal orientation;
    ///     `size.1` is also the width of the scrollbar
    /// -   `min_len`: minimum length for the whole bar
    ///
    /// Required bound: `min_len >= size.0`.
    pub fn scrollbar(&self) -> (Size, i32) {
        self.0.scrollbar()
    }

    /// Dimensions for a slider
    ///
    /// Returns:
    ///
    /// -   `size`: minimum size of handle in horizontal orientation;
    ///     `size.1` is also the width of the slider
    /// -   `min_len`: minimum length for the whole bar
    ///
    /// Required bound: `min_len >= size.0`.
    pub fn slider(&self) -> (Size, i32) {
        self.0.slider()
    }

    /// Dimensions for a progress bar
    ///
    /// Returns the minimum size for a horizontal progress bar. It is assumed
    /// that the width is adjustable while the height is (preferably) not.
    /// For a vertical bar, the values are swapped.
    pub fn progress_bar(&self) -> Size {
        self.0.progress_bar()
    }
}

/// A handle to the active theme, used for sizing
#[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
#[cfg_attr(doc_cfg, doc(cfg(internal_doc)))]
#[autoimpl(for<S: trait + ?Sized, R: Deref<Target = S>> R)]
pub trait SizeHandle {
    /// Get the scale (DPI) factor
    fn scale_factor(&self) -> f32;

    /// Convert a size in font Points to physical pixels
    fn pixels_from_points(&self, pt: f32) -> f32;

    /// Convert a size in font Em to physical pixels
    ///
    /// (This depends on the font size.)
    fn pixels_from_em(&self, em: f32) -> f32;

    /// Size of a frame around another element
    fn frame(&self, style: FrameStyle, is_vert: bool) -> FrameRules;

    /// Size of a separator frame between items
    fn separator(&self) -> Size;

    /// The margin around content within a widget
    ///
    /// Though inner margins are *usually* empty, they are sometimes drawn to,
    /// for example focus indicators.
    fn inner_margin(&self) -> Size;

    /// The margin between UI elements, where desired
    ///
    /// Widgets must not draw in outer margins.
    fn outer_margins(&self) -> Margins;

    /// The margin around text elements
    ///
    /// Similar to [`Self::outer_margins`], but intended for things like text
    /// labels which do not have a visible hard edge.
    fn text_margins(&self) -> Margins;

    /// The height of a line of text
    fn line_height(&self, class: TextClass) -> i32;

    /// Update a text object, setting font properties and getting a size bound
    ///
    /// This method updates the text's [`Environment`] and uses the result to
    /// calculate size requirements.
    ///
    /// It is necessary to update the environment *again* once the target `rect`
    /// is known: use [`Self::text_set_size`] to do this.
    ///
    /// [`Environment`]: crate::text::Environment
    fn text_bound(&self, text: &mut dyn TextApi, class: TextClass, axis: AxisInfo) -> SizeRules;

    /// Update a text object, setting font properties and wrap size
    ///
    /// Returns required size.
    fn text_set_size(
        &self,
        text: &mut dyn TextApi,
        class: TextClass,
        size: Size,
        align: (Align, Align),
    ) -> Vec2;

    /// Size of the element drawn by [`DrawMgr::checkbox`].
    fn checkbox(&self) -> Size;

    /// Size of the element drawn by [`DrawMgr::radiobox`].
    fn radiobox(&self) -> Size;

    /// A simple mark
    fn mark(&self, style: MarkStyle, is_vert: bool) -> SizeRules;

    /// Dimensions for a scrollbar
    ///
    /// Returns:
    ///
    /// -   `size`: minimum size of handle in horizontal orientation;
    ///     `size.1` is also the width of the scrollbar
    /// -   `min_len`: minimum length for the whole bar
    ///
    /// Required bound: `min_len >= size.0`.
    fn scrollbar(&self) -> (Size, i32);

    /// Dimensions for a slider
    ///
    /// Returns:
    ///
    /// -   `size`: minimum size of handle in horizontal orientation;
    ///     `size.1` is also the width of the slider
    /// -   `min_len`: minimum length for the whole bar
    ///
    /// Required bound: `min_len >= size.0`.
    fn slider(&self) -> (Size, i32);

    /// Dimensions for a progress bar
    ///
    /// Returns the minimum size for a horizontal progress bar. It is assumed
    /// that the width is adjustable while the height is (preferably) not.
    /// For a vertical bar, the values are swapped.
    fn progress_bar(&self) -> Size;
}
