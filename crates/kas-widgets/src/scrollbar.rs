// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! `ScrollBar` control

use std::fmt::Debug;

use super::{DragHandle, ScrollRegion};
use kas::event::{MsgPressFocus, Scroll};
use kas::prelude::*;

impl_scope! {
    /// A scroll bar
    ///
    /// Scroll bars allow user-input of a value between 0 and a defined maximum,
    /// and allow the size of the handle to be specified.
    ///
    /// # Messages
    ///
    /// On value change, pushes a value of type `i32`.
    #[derive(Clone, Debug, Default)]
    #[widget{
        hover_highlight = true;
    }]
    pub struct ScrollBar<D: Directional> {
        core: widget_core!(),
        direction: D,
        // Terminology assumes vertical orientation:
        width: i32,
        min_handle_len: i32,
        handle_len: i32,
        handle_value: i32, // contract: > 0
        max_value: i32,
        value: i32,
        #[widget]
        handle: DragHandle,
    }

    impl Self where D: Default {
        /// Construct a scroll bar
        ///
        /// Default values are assumed for all parameters.
        pub fn new() -> Self {
            ScrollBar::new_with_direction(D::default())
        }
    }

    impl Self {
        /// Construct a scroll bar with the given direction
        ///
        /// Default values are assumed for all parameters.
        #[inline]
        pub fn new_with_direction(direction: D) -> Self {
            ScrollBar {
                core: Default::default(),
                direction,
                width: 0,
                min_handle_len: 0,
                handle_len: 0,
                handle_value: 1,
                max_value: 0,
                value: 0,
                handle: DragHandle::new(),
            }
        }

        /// Set the initial page length
        ///
        /// See [`ScrollBar::set_limits`].
        #[inline]
        #[must_use]
        pub fn with_limits(mut self, max_value: i32, handle_value: i32) -> Self {
            let _ = self.set_limits(max_value, handle_value);
            self
        }

        /// Set the initial value
        #[inline]
        #[must_use]
        pub fn with_value(mut self, value: i32) -> Self {
            self.value = value.clamp(0, self.max_value);
            self
        }

        /// Set the page limits
        ///
        /// The `max_value` parameter specifies the maximum possible value.
        /// (The minimum is always 0.) For a scroll region, this should correspond
        /// to the maximum possible offset.
        ///
        /// The `handle_value` parameter specifies the size of the handle relative to
        /// the maximum value: the handle size relative to the length of the scroll
        /// bar is `handle_value / (max_value + handle_value)`. For a scroll region,
        /// this should correspond to the size of the visible region.
        /// The minimum value is 1.
        ///
        /// The choice of units is not important (e.g. can be pixels or lines),
        /// so long as both parameters use the same units.
        ///
        /// Returns [`TkAction::REDRAW`] if a redraw is required.
        pub fn set_limits(&mut self, max_value: i32, handle_value: i32) -> TkAction {
            // We should gracefully handle zero, though appearance may be wrong.
            self.handle_value = handle_value.max(1);

            self.max_value = max_value.max(0);
            self.value = self.value.clamp(0, self.max_value);
            self.update_widgets()
        }

        /// Read the current max value
        ///
        /// See also the [`ScrollBar::set_limits`] documentation.
        #[inline]
        pub fn max_value(&self) -> i32 {
            self.max_value
        }

        /// Read the current handle value
        ///
        /// See also the [`ScrollBar::set_limits`] documentation.
        #[inline]
        pub fn handle_value(&self) -> i32 {
            self.handle_value
        }

        /// Get the current value
        #[inline]
        pub fn value(&self) -> i32 {
            self.value
        }

        /// Set the value
        pub fn set_value(&mut self, value: i32) -> TkAction {
            let value = value.clamp(0, self.max_value);
            if value == self.value {
                TkAction::empty()
            } else {
                self.value = value;
                self.handle.set_offset(self.offset()).1
            }
        }

        #[inline]
        fn bar_len(&self) -> i32 {
            match self.direction.is_vertical() {
                false => self.core.rect.size.0,
                true => self.core.rect.size.1,
            }
        }

        fn update_widgets(&mut self) -> TkAction {
            let len = self.bar_len();
            let total = i64::from(self.max_value) + i64::from(self.handle_value);
            let handle_len = i64::from(self.handle_value) * i64::conv(len) / total;
            self.handle_len = i32::conv(handle_len).max(self.min_handle_len).min(len);
            let mut size = self.core.rect.size;
            if self.direction.is_horizontal() {
                size.0 = self.handle_len;
            } else {
                size.1 = self.handle_len;
            }
            self.handle.set_size_and_offset(size, self.offset())
        }

        // translate value to offset in local coordinates
        fn offset(&self) -> Offset {
            let len = self.bar_len() - self.handle_len;
            let lhs = i64::from(self.value) * i64::conv(len);
            let rhs = i64::from(self.max_value);
            let mut pos = if rhs == 0 {
                0
            } else {
                i32::conv((lhs + (rhs / 2)) / rhs).min(len)
            };
            if self.direction.is_reversed() {
                pos = len - pos;
            }
            match self.direction.is_vertical() {
                false => Offset(pos, 0),
                true => Offset(0, pos),
            }
        }

        // true if not equal to old value
        fn set_offset(&mut self, offset: Offset) -> bool {
            let len = self.bar_len() - self.handle_len;
            let mut offset = match self.direction.is_vertical() {
                false => offset.0,
                true => offset.1,
            };
            if self.direction.is_reversed() {
                offset = len - offset;
            }

            let lhs = i64::from(offset) * i64::from(self.max_value);
            let rhs = i64::conv(len);
            if rhs == 0 {
                debug_assert_eq!(self.value, 0);
                return false;
            }
            let value = i32::conv((lhs + (rhs / 2)) / rhs);
            let value = value.clamp(0, self.max_value);
            if value != self.value {
                self.value = value;
                return true;
            }
            false
        }
    }

    impl Layout for Self {
        fn size_rules(&mut self, size_mgr: SizeMgr, axis: AxisInfo) -> SizeRules {
            let (size, min_len) = size_mgr.scrollbar();
            let margins = (0, 0);
            if self.direction.is_vertical() == axis.is_vertical() {
                SizeRules::new(min_len, min_len, margins, Stretch::High)
            } else {
                self.width = size.1;
                SizeRules::fixed(size.1, margins)
            }
        }

        fn set_rect(&mut self, mgr: &mut SetRectMgr, rect: Rect, align: AlignHints) {
            let mut ideal_size = Size::splat(self.width);
            ideal_size.set_component(self.direction, i32::MAX);
            let rect = align
                .complete(Align::Center, Align::Center)
                .aligned_rect(ideal_size, rect);
            self.core.rect = rect;
            self.handle.set_rect(mgr, rect, align);
            self.min_handle_len = (mgr.size_mgr().scrollbar().0).0;
            let _ = self.update_widgets();
        }

        fn find_id(&mut self, coord: Coord) -> Option<WidgetId> {
            if !self.rect().contains(coord) {
                return None;
            }
            self.handle.find_id(coord).or(Some(self.id()))
        }

        fn draw(&mut self, mut draw: DrawMgr) {
            let dir = self.direction.as_direction();
            draw.scrollbar(self.rect(), &self.handle, dir);
        }
    }

    impl Widget for Self {
        fn handle_event(&mut self, mgr: &mut EventMgr, event: Event) -> Response {
            match event {
                Event::PressStart { source, coord, .. } => {
                    let offset = self.handle.handle_press_on_track(mgr, source, coord);
                    let (offset, action) = self.handle.set_offset(offset);
                    *mgr |= action;
                    if self.set_offset(offset) {
                        mgr.push_msg(self.value);
                    }
                    Response::Used
                }
                _ => Response::Unused
            }
        }

        fn handle_message(&mut self, mgr: &mut EventMgr, _: usize) {
            if let Some(MsgPressFocus) = mgr.try_pop_msg() {
                // Useless to us, but we should remove it.
            } else if let Some(offset) = mgr.try_pop_msg() {
                let (offset, action) = self.handle.set_offset(offset);
                *mgr |= action;
                if self.set_offset(offset) {
                    mgr.push_msg(self.value);
                }
            }
        }
    }
}

/// Additional functionality on scrollable widgets
///
/// This trait should be implemented by widgets supporting scrolling, enabling
/// a parent (such as the [`ScrollBars`] wrapper) to add controls.
///
/// If the widget scrolls itself it should set a scroll action via [`EventMgr::set_scroll`].
pub trait Scrollable: Widget {
    /// Given size `size`, returns whether `(horiz, vert)` scrolling is required
    fn scroll_axes(&self, size: Size) -> (bool, bool);

    /// Get the maximum scroll offset
    ///
    /// Note: the minimum scroll offset is always zero.
    fn max_scroll_offset(&self) -> Offset;

    /// Get the current scroll offset
    ///
    /// Contents of the scroll region are translated by this offset (to convert
    /// coordinates from the outer region to the scroll region, add this offset).
    ///
    /// The offset is restricted between [`Offset::ZERO`] and
    /// [`ScrollRegion::max_scroll_offset`].
    fn scroll_offset(&self) -> Offset;

    /// Set the scroll offset
    ///
    /// This may be used for programmatic scrolling, e.g. by a wrapping widget
    /// with scroll controls. Note that calling this method directly on the
    /// scrolling widget will not update any controls in a wrapping widget.
    ///
    /// The offset is clamped to the available scroll range and applied. The
    /// resulting offset is returned.
    fn set_scroll_offset(&mut self, mgr: &mut EventMgr, offset: Offset) -> Offset;
}

impl_scope! {
    /// A scrollable region with bars
    ///
    /// This is essentially a `ScrollBars<ScrollRegion<W>>`:
    /// [`ScrollRegion`] handles the actual scrolling and wheel/touch events,
    /// while [`ScrollBars`] adds scrollbar controls.
    #[autoimpl(Deref, DerefMut using self.0)]
    #[autoimpl(class_traits using self.0 where W: trait)]
    #[derive(Clone, Debug, Default)]
    #[widget{
        derive = self.0;
    }]
    pub struct ScrollBarRegion<W: Widget>(ScrollBars<ScrollRegion<W>>);

    impl Self {
        /// Construct a `ScrollBarRegion<W>`
        #[inline]
        pub fn new(inner: W) -> Self {
            ScrollBarRegion(ScrollBars::new(ScrollRegion::new(inner)))
        }

        /// Auto-enable bars
        ///
        /// If enabled (default), this automatically enables/disables scroll bars
        /// as required when resized.
        ///
        /// This has the side-effect of reserving enough space for scroll bars even
        /// when not required.
        #[inline]
        #[must_use]
        pub fn with_auto_bars(self, enable: bool) -> Self {
            ScrollBarRegion(self.0.with_auto_bars(enable))
        }

        /// Set which scroll bars are visible
        ///
        /// Calling this method also disables automatic scroll bars.
        #[inline]
        #[must_use]
        pub fn with_bars(self, horiz: bool, vert: bool) -> Self {
            ScrollBarRegion(self.0.with_bars(horiz, vert))
        }

        /// Set which scroll bars are visible
        ///
        /// Calling this method also disables automatic scroll bars.
        /// A resize is required to update the child and scrollbar widgets.
        #[inline]
        pub fn set_bars(&mut self, horiz: bool, vert: bool) -> TkAction {
            self.0.set_bars(horiz, vert)
        }

        /// Query which scroll bars are visible
        ///
        /// Returns `(horiz, vert)` tuple.
        #[inline]
        pub fn bars(&self) -> (bool, bool) {
            self.0.bars()
        }

        /// Access inner widget directly
        #[inline]
        pub fn inner(&self) -> &W {
            self.0.inner.inner()
        }

        /// Access inner widget directly
        #[inline]
        pub fn inner_mut(&mut self) -> &mut W {
            self.0.inner.inner_mut()
        }
    }

    impl Scrollable for Self {
        #[inline]
        fn scroll_axes(&self, size: Size) -> (bool, bool) {
            self.0.inner.scroll_axes(size)
        }
        #[inline]
        fn max_scroll_offset(&self) -> Offset {
            self.0.inner.max_scroll_offset()
        }
        #[inline]
        fn scroll_offset(&self) -> Offset {
            self.0.inner.scroll_offset()
        }
        #[inline]
        fn set_scroll_offset(&mut self, mgr: &mut EventMgr, offset: Offset) -> Offset {
            self.0.set_scroll_offset(mgr, offset)
        }
    }
}

impl_scope! {
    /// Scrollbar controls
    ///
    /// This is a wrapper adding scrollbar controls around a child. Note that this
    /// widget does not enable scrolling; see [`ScrollBarRegion`] for that.
    ///
    /// Scrollbar positioning does not respect the inner widgets margins, since
    /// the result looks poor when content is scrolled. Instead the content should
    /// force internal margins by wrapping contents with a (zero-sized) frame.
    /// [`ScrollRegion`] already does this.
    #[autoimpl(Deref, DerefMut using self.inner)]
    #[autoimpl(class_traits using self.inner where W: trait)]
    #[derive(Clone, Debug, Default)]
    #[widget]
    pub struct ScrollBars<W: Scrollable> {
        core: widget_core!(),
        auto_bars: bool,
        show_bars: (bool, bool),
        #[widget]
        horiz_bar: ScrollBar<kas::dir::Right>,
        #[widget]
        vert_bar: ScrollBar<kas::dir::Down>,
        #[widget]
        inner: W,
    }

    impl Self {
        /// Construct
        ///
        /// By default scrollbars are automatically enabled based on requirements.
        /// See [`ScrollBars::with_auto_bars`] and [`ScrollBars::with_bars`].
        #[inline]
        pub fn new(inner: W) -> Self {
            ScrollBars {
                core: Default::default(),
                auto_bars: true,
                show_bars: (false, false),
                horiz_bar: ScrollBar::new(),
                vert_bar: ScrollBar::new(),
                inner,
            }
        }

        /// Auto-enable bars
        ///
        /// If enabled (default), this automatically enables/disables scroll bars
        /// as required when resized.
        ///
        /// This has the side-effect of reserving enough space for scroll bars even
        /// when not required.
        #[inline]
        #[must_use]
        pub fn with_auto_bars(mut self, enable: bool) -> Self {
            self.auto_bars = enable;
            self
        }

        /// Set which scroll bars are visible
        ///
        /// Calling this method also disables automatic scroll bars.
        #[inline]
        #[must_use]
        pub fn with_bars(mut self, horiz: bool, vert: bool) -> Self {
            self.auto_bars = false;
            self.show_bars = (horiz, vert);
            self
        }

        /// Set which scroll bars are visible
        ///
        /// Calling this method also disables automatic scroll bars.
        /// A resize is required to update the child and scrollbar widgets.
        #[inline]
        pub fn set_bars(&mut self, horiz: bool, vert: bool) -> TkAction {
            self.auto_bars = false;
            self.show_bars = (horiz, vert);
            TkAction::RESIZE
        }

        /// Query which scroll bars are visible
        ///
        /// Returns `(horiz, vert)` tuple.
        #[inline]
        pub fn bars(&self) -> (bool, bool) {
            self.show_bars
        }

        /// Access inner widget directly
        #[inline]
        pub fn inner(&self) -> &W {
            &self.inner
        }

        /// Access inner widget directly
        #[inline]
        pub fn inner_mut(&mut self) -> &mut W {
            &mut self.inner
        }

        fn draw_(&mut self, mut draw: DrawMgr) {
            if self.show_bars.0 {
                draw.recurse(&mut self.horiz_bar);
            }
            if self.show_bars.1 {
                draw.recurse(&mut self.vert_bar);
            }
            draw.recurse(&mut self.inner);
        }
    }

    impl Scrollable for Self {
        #[inline]
        fn scroll_axes(&self, size: Size) -> (bool, bool) {
            self.inner.scroll_axes(size)
        }
        #[inline]
        fn max_scroll_offset(&self) -> Offset {
            self.inner.max_scroll_offset()
        }
        #[inline]
        fn scroll_offset(&self) -> Offset {
            self.inner.scroll_offset()
        }
        fn set_scroll_offset(&mut self, mgr: &mut EventMgr, offset: Offset) -> Offset {
            let offset = self.inner.set_scroll_offset(mgr, offset);
            *mgr |= self.horiz_bar.set_value(offset.0) | self.vert_bar.set_value(offset.1);
            offset
        }
    }

    impl Layout for Self {
        fn size_rules(&mut self, size_mgr: SizeMgr, axis: AxisInfo) -> SizeRules {
            let mut rules = self.inner.size_rules(size_mgr.re(), axis);
            if axis.is_horizontal() && (self.auto_bars || self.show_bars.1) {
                rules.append(self.vert_bar.size_rules(size_mgr.re(), axis));
            } else if axis.is_vertical() && (self.auto_bars || self.show_bars.0) {
                rules.append(self.horiz_bar.size_rules(size_mgr.re(), axis));
            }
            rules
        }

        fn set_rect(&mut self, mgr: &mut SetRectMgr, rect: Rect, align: AlignHints) {
            self.core.rect = rect;
            let pos = rect.pos;
            let mut child_size = rect.size;

            let bar_width = (mgr.size_mgr().scrollbar().0).1;
            if self.auto_bars {
                self.show_bars = self.inner.scroll_axes(child_size);
            }
            if self.show_bars.0 {
                child_size.1 -= bar_width;
            }
            if self.show_bars.1 {
                child_size.0 -= bar_width;
            }

            let child_rect = Rect::new(pos, child_size);
            self.inner.set_rect(mgr, child_rect, align);
            let max_scroll_offset = self.inner.max_scroll_offset();

            if self.show_bars.0 {
                let pos = Coord(pos.0, rect.pos2().1 - bar_width);
                let size = Size::new(child_size.0, bar_width);
                self.horiz_bar
                    .set_rect(mgr, Rect { pos, size }, AlignHints::NONE);
                let _ = self.horiz_bar.set_limits(max_scroll_offset.0, rect.size.0);
            }
            if self.show_bars.1 {
                let pos = Coord(rect.pos2().0 - bar_width, pos.1);
                let size = Size::new(bar_width, self.core.rect.size.1);
                self.vert_bar
                    .set_rect(mgr, Rect { pos, size }, AlignHints::NONE);
                let _ = self.vert_bar.set_limits(max_scroll_offset.1, rect.size.1);
            }
        }

        fn find_id(&mut self, coord: Coord) -> Option<WidgetId> {
            if !self.rect().contains(coord) {
                return None;
            }
            self.vert_bar.find_id(coord)
                .or_else(|| self.horiz_bar.find_id(coord))
                .or_else(|| self.inner.find_id(coord))
                .or(Some(self.id()))
        }

        #[cfg(feature = "min_spec")]
        default fn draw(&mut self, draw: DrawMgr) {
            self.draw_(draw);
        }
        #[cfg(not(feature = "min_spec"))]
        fn draw(&mut self, draw: DrawMgr) {
            self.draw_(draw);
        }
    }

    #[cfg(feature = "min_spec")]
    impl<W: Widget> Layout for ScrollBars<ScrollRegion<W>> {
        fn draw(&mut self, mut draw: DrawMgr) {
            // Enlarge clip region to *our* rect:
            draw.with_clip_region(self.core.rect, self.inner.scroll_offset(), |mut draw| {
                draw.recurse(&mut self.inner);
            });
            // Use a second clip region to force draw order:
            draw.with_clip_region(self.core.rect, Offset::ZERO, |mut draw| {
                if self.show_bars.0 {
                    draw.recurse(&mut self.horiz_bar);
                }
                if self.show_bars.1 {
                    draw.recurse(&mut self.vert_bar);
                }
            });
        }
    }

    impl Widget for Self {
        fn configure(&mut self, mgr: &mut SetRectMgr) {
            mgr.register_nav_fallback(self.id());
        }

        fn handle_message(&mut self, mgr: &mut EventMgr, index: usize) {
            if index == widget_index![self.horiz_bar] {
                if let Some(msg) = mgr.try_pop_msg() {
                    let offset = Offset(msg, self.inner.scroll_offset().1);
                    self.inner.set_scroll_offset(mgr, offset);
                }
            } else if index == widget_index![self.vert_bar] {
                if let Some(msg) = mgr.try_pop_msg() {
                    let offset = Offset(self.inner.scroll_offset().0, msg);
                    self.inner.set_scroll_offset(mgr, offset);
                }
            }
        }

        fn handle_scroll(&mut self, mgr: &mut EventMgr, _: Scroll) {
            // We assume the inner already updated its positions; this is just to set bars
            let offset = self.inner.scroll_offset();
            *mgr |= self.horiz_bar.set_value(offset.0) | self.vert_bar.set_value(offset.1);
        }
    }
}
