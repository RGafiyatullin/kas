// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Size reservation

use kas::prelude::*;

/// Parameterisation of [`Reserve`] using a function pointer
///
/// Since it is impossible to name closures, using [`Reserve`] where a type is
/// required (e.g. in a struct field) is only possible by making the containing
/// struct generic over this field, which may be undesirable. As an alternative
/// a function pointer may be preferred.
pub type ReserveP<W> = Reserve<W, fn(SizeMgr, AxisInfo) -> SizeRules>;

impl_scope! {
    /// A generic widget for size reservations
    ///
    /// In a few cases it is desirable to reserve more space for a widget than
    /// required for the current content, e.g. if a label's text may change. This
    /// widget can be used for this by wrapping the base widget.
    #[autoimpl(Debug ignore self.reserve)]
    #[autoimpl(Deref, DerefMut using self.inner)]
    #[autoimpl(class_traits using self.inner where W: trait)]
    #[derive(Clone, Default)]
    #[widget{ layout = self.inner; }]
    pub struct Reserve<W: Widget, R: FnMut(SizeMgr, AxisInfo) -> SizeRules + 'static> {
        core: widget_core!(),
        #[widget]
        pub inner: W,
        reserve: R,
    }

    impl Self {
        /// Construct a reserve
        ///
        /// The closure `reserve` should generate `SizeRules` on request, just like
        /// [`Layout::size_rules`]. This can be done by instantiating a temporary
        /// widget, for example:
        ///```
        /// use kas_widgets::adapter::Reserve;
        /// use kas_widgets::Label;
        /// use kas::prelude::*;
        ///
        /// let label = Reserve::new(Label::new("0"), |size_mgr, axis| {
        ///     Label::new("00000").size_rules(size_mgr, axis)
        /// });
        ///```
        /// Alternatively one may use virtual pixels:
        ///```
        /// use kas_widgets::adapter::Reserve;
        /// use kas_widgets::Filler;
        /// use kas::prelude::*;
        ///
        /// let label = Reserve::new(Filler::new(), |size_mgr, axis| {
        ///     let size = i32::conv_ceil(size_mgr.scale_factor() * 100.0);
        ///     SizeRules::fixed(size, (0, 0))
        /// });
        ///```
        /// The resulting `SizeRules` will be the max of those for the inner widget
        /// and the result of the `reserve` closure.
        #[inline]
        pub fn new(inner: W, reserve: R) -> Self {
            Reserve {
                core: Default::default(),
                inner,
                reserve,
            }
        }
    }

    impl Layout for Self {
        fn size_rules(&mut self, size_mgr: SizeMgr, axis: AxisInfo) -> SizeRules {
            let inner_rules = self.inner.size_rules(size_mgr.re(), axis);
            let reserve_rules = (self.reserve)(size_mgr.re(), axis);
            inner_rules.max(reserve_rules)
        }
    }
}
