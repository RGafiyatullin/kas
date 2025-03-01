// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! A tabbed stack

use crate::{Row, Stack, TextButton};
use kas::prelude::*;
use std::fmt::Debug;

#[derive(Clone, Debug)]
struct MsgSelect;

#[derive(Clone, Debug)]
struct MsgSelectIndex(usize);

/// A tab
///
/// TODO: a tab is not a button! Support directional graphics, icon and close button.
pub type Tab = TextButton;

/// A tabbed stack of boxed widgets
///
/// This is a parametrisation of [`TabStack`].
pub type BoxTabStack = TabStack<Box<dyn Widget>>;

impl_scope! {
    /// A tabbed stack of widgets
    ///
    /// A stack consists a set of child widgets, "pages", all of equal size.
    /// Only a single page is visible at a time. The page is "turned" via tab
    /// handles or calling [`Self::set_active`].
    ///
    /// Type parameter `D` controls the position of tabs relative to the stack;
    /// default value is [`Direction::Up`]: tabs are positioned above the stack.
    /// Within the bar, items are always left-to-right
    /// (TODO: support for vertical bars).
    ///
    /// This may only be parametrised with a single widget type, thus usually
    /// it will be necessary to box children (this is what [`BoxTabStack`] is).
    ///
    /// See also the main implementing widget: [`Stack`].
    #[impl_default]
    #[derive(Clone, Debug)]
    #[widget {
        layout = list(self.direction): [
            self.stack,
            self.tabs,
        ];
    }]
    pub struct TabStack<W: Widget> {
        core: widget_core!(),
        direction: Direction = Direction::Up,
        #[widget]
        tabs: Row<Tab>, // TODO: want a TabBar widget for scrolling support?
        #[widget]
        stack: Stack<W>,
    }

    impl Self {
        /// Construct a new, empty instance
        pub fn new() -> Self {
            Self {
                core: Default::default(),
                direction: Direction::Up,
                stack: Stack::new(),
                tabs: Row::new().on_message(|mgr, index| {
                    if let Some(MsgSelect) = mgr.try_pop_msg() {
                        mgr.push_msg(MsgSelectIndex(index));
                    }
                }),
            }
        }

        /// Set the position of tabs relative to content
        ///
        /// Default value: [`Direction::Up`]
        pub fn set_direction(&mut self, direction: Direction) -> TkAction {
            self.direction = direction;
            // Note: most of the time SET_SIZE would be enough, but margins can be different
            TkAction::RESIZE
        }
    }

    impl Widget for Self {
        fn handle_message(&mut self, mgr: &mut EventMgr, _: usize) {
            if let Some(MsgSelectIndex(index)) = mgr.try_pop_msg() {
                mgr.set_rect_mgr(|mgr| self.set_active(mgr, index));
            }
        }
    }
}

impl<W: Widget> TabStack<W> {
    /// Limit the number of pages considered by [`Layout::size_rules`]
    ///
    /// By default, this is `usize::MAX`: all pages affect the result. If
    /// this is set to 1 then only the active page will affect the result. If
    /// this is `n > 1`, then `min(n, num_pages)` pages (including active)
    /// will be used. (If this is set to 0 it is silently replaced with 1.)
    ///
    /// Using a limit lower than the number of pages has two effects:
    /// (1) resizing is faster and (2) calling [`Self::set_active`] may cause a
    /// full-window resize.
    pub fn set_size_limit(&mut self, limit: usize) {
        self.stack.set_size_limit(limit);
    }

    /// Get the index of the active page
    #[inline]
    pub fn active(&self) -> usize {
        self.stack.active()
    }

    /// Set the active page (inline)
    ///
    /// Unlike [`Self::set_active`], this does not update anything; it is
    /// assumed that sizing happens afterwards.
    #[inline]
    pub fn with_active(mut self, active: usize) -> Self {
        self.stack = self.stack.with_active(active);
        self
    }

    /// Set the active page
    ///
    /// Behaviour depends on whether [`SizeRules`] were already solved for
    /// `index` (see [`Self::set_size_limit`] and note that methods like
    /// [`Self::push`] do not solve rules for new pages). Case:
    ///
    /// -   `index >= num_pages`: no page displayed
    /// -   `index == active` and `SizeRules` were solved: nothing happens
    /// -   `SizeRules` were solved: set layout ([`Layout::set_rect`]) and
    ///     update mouse-cursor target ([`TkAction::REGION_MOVED`])
    /// -   Otherwise: resize the whole window ([`TkAction::RESIZE`])
    pub fn set_active(&mut self, mgr: &mut SetRectMgr, index: usize) {
        self.stack.set_active(mgr, index);
    }

    /// Get a direct reference to the active child widget, if any
    pub fn get_active(&self) -> Option<&W> {
        self.stack.get_active()
    }

    /// True if there are no pages
    pub fn is_empty(&self) -> bool {
        self.stack.is_empty()
    }

    /// Returns the number of pages
    pub fn len(&self) -> usize {
        self.stack.len()
    }

    /// Remove all pages
    ///
    /// This does not change the active page index.
    pub fn clear(&mut self) {
        self.stack.clear();
        self.tabs.clear();
    }

    /// Get a page
    pub fn get(&self, index: usize) -> Option<&W> {
        self.stack.get(index)
    }

    /// Get a page
    pub fn get_mut(&mut self, index: usize) -> Option<&mut W> {
        self.stack.get_mut(index)
    }

    /// Get a tab
    pub fn get_tab(&self, index: usize) -> Option<&Tab> {
        self.tabs.get(index)
    }

    /// Get a tab
    pub fn get_tab_mut(&mut self, index: usize) -> Option<&mut Tab> {
        self.tabs.get_mut(index)
    }

    /// Append a page (inline)
    ///
    /// Does not configure or size child.
    pub fn with_tab(mut self, tab: Tab, widget: W) -> Self {
        let _ = self.stack.edit(|widgets| widgets.push(widget));
        let _ = self.tabs.edit(|tabs| tabs.push(tab));
        self
    }

    /// Append a page (inline)
    ///
    /// Does not configure or size child.
    pub fn with_title(self, title: impl Into<AccelString>, widget: W) -> Self {
        self.with_tab(Tab::new_on(title, |mgr| mgr.push_msg(MsgSelect)), widget)
    }

    /// Append a page
    ///
    /// The new page is configured immediately. If it becomes the active page
    /// and then [`TkAction::RESIZE`] will be triggered.
    ///
    /// Returns the new page's index.
    pub fn push(&mut self, mgr: &mut SetRectMgr, tab: Tab, widget: W) -> usize {
        let ti = self.tabs.push(mgr, tab);
        let si = self.stack.push(mgr, widget);
        debug_assert_eq!(ti, si);
        si
    }

    /// Remove the last child widget (if any) and return
    ///
    /// If this page was active then the previous page becomes active.
    pub fn pop(&mut self, mgr: &mut SetRectMgr) -> Option<(Tab, W)> {
        let tab = self.tabs.pop(mgr);
        let w = self.stack.pop(mgr);
        debug_assert_eq!(tab.is_some(), w.is_some());
        tab.zip(w)
    }

    /// Inserts a child widget position `index`
    ///
    /// Panics if `index > len`.
    ///
    /// The new child is configured immediately. The active page does not
    /// change.
    pub fn insert(&mut self, mgr: &mut SetRectMgr, index: usize, tab: Tab, widget: W) {
        self.tabs.insert(mgr, index, tab);
        self.stack.insert(mgr, index, widget);
    }

    /// Removes the child widget at position `index`
    ///
    /// Panics if `index` is out of bounds.
    ///
    /// If the active page is removed then the previous page (if any) becomes
    /// active.
    pub fn remove(&mut self, mgr: &mut SetRectMgr, index: usize) -> (Tab, W) {
        let tab = self.tabs.remove(mgr, index);
        let stack = self.stack.remove(mgr, index);
        (tab, stack)
    }

    /// Replace the child at `index`
    ///
    /// Panics if `index` is out of bounds.
    ///
    /// The new child is configured immediately. If it replaces the active page,
    /// then [`TkAction::RESIZE`] is triggered.
    pub fn replace(&mut self, mgr: &mut SetRectMgr, index: usize, w: W) -> W {
        self.stack.replace(mgr, index, w)
    }

    /// Append child widgets from an iterator
    ///
    /// New children are configured immediately. If a new page becomes active,
    /// then [`TkAction::RESIZE`] is triggered.
    pub fn extend<T: IntoIterator<Item = (Tab, W)>>(&mut self, mgr: &mut SetRectMgr, iter: T) {
        let iter = iter.into_iter();
        // let min_len = iter.size_hint().0;
        // self.tabs.reserve(min_len);
        // self.stack.reserve(min_len);
        for (tab, w) in iter {
            self.tabs.push(mgr, tab);
            self.stack.push(mgr, w);
        }
    }
}

impl<W: Widget, T: IntoIterator<Item = (Tab, W)>> From<T> for TabStack<W> {
    #[inline]
    fn from(iter: T) -> Self {
        let iter = iter.into_iter();
        let min_len = iter.size_hint().0;
        let mut stack = Vec::with_capacity(min_len);
        let mut tabs = Vec::with_capacity(min_len);
        for (tab, w) in iter {
            stack.push(w);
            tabs.push(tab);
        }
        Self {
            stack: Stack::new_vec(stack),
            tabs: Row::new_vec(tabs),
            ..Default::default()
        }
    }
}
