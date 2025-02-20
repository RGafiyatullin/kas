// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! A row or column with run-time adjustable contents

use kas::dir::{Down, Right};
use kas::{layout, prelude::*};
use std::collections::hash_map::{Entry, HashMap};
use std::ops::{Index, IndexMut};

/// A generic row widget
///
/// See documentation of [`List`] type.
pub type Row<W> = List<Right, W>;

/// A generic column widget
///
/// See documentation of [`List`] type.
pub type Column<W> = List<Down, W>;

/// A row of boxed widgets
///
/// See documentation of [`List`] type.
pub type BoxRow = BoxList<Right>;

/// A column of boxed widgets
///
/// See documentation of [`List`] type.
pub type BoxColumn = BoxList<Down>;

/// A row/column of boxed widgets
///
/// This is parameterised over directionality.
///
/// See documentation of [`List`] type.
pub type BoxList<D> = List<D, Box<dyn Widget>>;

impl_scope! {
    /// A generic row/column widget
    ///
    /// This type is roughly [`Vec`] but for widgets. Generics:
    ///
    /// -   `D:` [`Directional`] — fixed or run-time direction of layout
    /// -   `W:` [`Widget`] — type of widget
    ///
    /// ## Alternatives
    ///
    /// Some more specific type-defs are available:
    ///
    /// -   [`Row`] and [`Column`] fix the direction `D`
    /// -   [`BoxList`] fixes the widget type to `Box<dyn Widget>`
    /// -   [`BoxRow`] and [`BoxColumn`] fix both type parameters
    ///
    /// ## Performance
    ///
    /// Configuring and resizing elements is O(n) in the number of children.
    /// Drawing and event handling is O(log n) in the number of children (assuming
    /// only a small number are visible at any one time).
    ///
    /// # Messages
    ///
    /// If a handler is specified via [`Self::on_message`] then this handler is
    /// called when a child pushes a message. This allows associating the
    /// child's index with a message.
    #[autoimpl(Clone where W: Clone)]
    #[autoimpl(Debug ignore self.on_message)]
    #[autoimpl(Default where D: Default)]
    #[widget {
        layout = slice(self.direction) 'layout: self.widgets;
    }]
    pub struct List<D: Directional, W: Widget> {
        core: widget_core!(),
        widgets: Vec<W>,
        direction: D,
        next: usize,
        id_map: HashMap<usize, usize>, // map key of WidgetId to index
        on_message: Option<fn(&mut EventMgr, usize)>,
    }

    impl WidgetChildren for Self {
        #[inline]
        fn num_children(&self) -> usize {
            self.widgets.len()
        }
        #[inline]
        fn get_child(&self, index: usize) -> Option<&dyn Widget> {
            self.widgets.get(index).map(|w| w.as_widget())
        }
        #[inline]
        fn get_child_mut(&mut self, index: usize) -> Option<&mut dyn Widget> {
            self.widgets.get_mut(index).map(|w| w.as_widget_mut())
        }

        fn find_child_index(&self, id: &WidgetId) -> Option<usize> {
            id.next_key_after(self.id_ref()).and_then(|k| self.id_map.get(&k).cloned())
        }
    }

    impl Widget for Self {
        fn make_child_id(&mut self, index: usize) -> WidgetId {
            if let Some(child) = self.widgets.get(index) {
                // Use the widget's existing identifier, if any
                if child.id_ref().is_valid() {
                    if let Some(key) = child.id_ref().next_key_after(self.id_ref()) {
                        self.id_map.insert(key, index);
                        return child.id();
                    }
                }
            }

            loop {
                let key = self.next;
                self.next += 1;
                if let Entry::Vacant(entry) = self.id_map.entry(key) {
                    entry.insert(index);
                    return self.id_ref().make_child(key);
                }
            }
        }

        fn pre_configure(&mut self, _: &mut SetRectMgr, id: WidgetId) {
            self.core.id = id;
            self.id_map.clear();
        }

        fn spatial_nav(
            &mut self,
            _: &mut SetRectMgr,
            reverse: bool,
            from: Option<usize>,
        ) -> Option<usize> {
            let reverse = reverse ^ self.direction.is_reversed();
            kas::util::spatial_nav(reverse, from, self.num_children())
        }

        fn handle_message(&mut self, mgr: &mut EventMgr, index: usize) {
            if let Some(f) = self.on_message {
                f(mgr, index);
            }
        }
    }

    impl Self where D: Default {
        /// Construct a new instance
        ///
        /// This constructor is available where the direction is determined by the
        /// type: for `D: Directional + Default`. In other cases, use
        /// [`Self::new_dir`].
        #[inline]
        pub fn new() -> Self {
            Self::new_vec(vec![])
        }

        /// Construct a new instance with vec
        ///
        /// This constructor is available where the direction is determined by the
        /// type: for `D: Directional + Default`. In other cases, use
        /// [`Self::new_dir_vec`].
        #[inline]
        pub fn new_vec(widgets: Vec<W>) -> Self {
            Self::new_dir_vec(D::default(), widgets)
        }
    }

    impl<W: Widget> List<Direction, W> {
        /// Set the direction of contents
        pub fn set_direction(&mut self, direction: Direction) -> TkAction {
            self.direction = direction;
            // Note: most of the time SET_SIZE would be enough, but margins can be different
            TkAction::RESIZE
        }
    }

    impl Self {
        /// Construct a new instance with explicit direction
        #[inline]
        pub fn new_dir(direction: D) -> Self {
            List::new_dir_vec(direction, vec![])
        }

        /// Construct a new instance with explicit direction and vec
        #[inline]
        pub fn new_dir_vec(direction: D, widgets: Vec<W>) -> Self {
            List {
                core: Default::default(),
                widgets,
                direction,
                next: 0,
                id_map: Default::default(),
                on_message: None,
            }
        }

        /// Assign a child message handler
        ///
        /// This handler (if any) is called when a child pushes a message:
        /// `f(mgr, index)`, where `index` is the child's index.
        #[inline]
        pub fn set_on_message(&mut self, f: Option<fn(&mut EventMgr, usize)>) {
            self.on_message = f;
        }

        /// Assign a child message handler (inline style)
        ///
        /// This handler is called when a child pushes a message:
        /// `f(mgr, index)`, where `index` is the child's index.
        #[inline]
        pub fn on_message(mut self, f: fn(&mut EventMgr, usize)) -> Self {
            self.on_message = Some(f);
            self
        }

        /// Edit the list of children directly
        ///
        /// This may be used to edit children before window construction. It may
        /// also be used from a running UI, but in this case a full reconfigure
        /// of the window's widgets is required (triggered by the the return
        /// value, [`TkAction::RECONFIGURE`]).
        #[inline]
        pub fn edit<F: FnOnce(&mut Vec<W>)>(&mut self, f: F) -> TkAction {
            f(&mut self.widgets);
            TkAction::RECONFIGURE
        }

        /// Get the direction of contents
        pub fn direction(&self) -> Direction {
            self.direction.as_direction()
        }

        /// Access layout storage
        ///
        /// The number of columns/rows is [`Self.len`].
        #[inline]
        pub fn layout_storage(&mut self) -> &mut impl layout::RowStorage {
            &mut self.core.layout
        }

        /// True if there are no child widgets
        pub fn is_empty(&self) -> bool {
            self.widgets.is_empty()
        }

        /// Returns the number of child widgets
        pub fn len(&self) -> usize {
            self.widgets.len()
        }

        /// Remove all child widgets
        pub fn clear(&mut self) {
            self.widgets.clear();
        }

        /// Returns a reference to the child, if any
        pub fn get(&self, index: usize) -> Option<&W> {
            self.widgets.get(index)
        }

        /// Returns a mutable reference to the child, if any
        pub fn get_mut(&mut self, index: usize) -> Option<&mut W> {
            self.widgets.get_mut(index)
        }

        /// Append a child widget
        ///
        /// The new child is configured immediately. [`TkAction::RESIZE`] is
        /// triggered.
        ///
        /// Returns the new element's index.
        pub fn push(&mut self, mgr: &mut SetRectMgr, widget: W) -> usize {
            let index = self.widgets.len();
            self.widgets.push(widget);
            let id = self.make_child_id(index);
            mgr.configure(id, &mut self.widgets[index]);
            *mgr |= TkAction::RESIZE;
            index
        }

        /// Remove the last child widget (if any) and return
        ///
        /// Triggers [`TkAction::RESIZE`].
        pub fn pop(&mut self, mgr: &mut SetRectMgr) -> Option<W> {
            let result = self.widgets.pop();
            if let Some(w) = result.as_ref() {
                *mgr |= TkAction::RESIZE;

                if w.id_ref().is_valid() {
                    if let Some(key) = w.id_ref().next_key_after(self.id_ref()) {
                        self.id_map.remove(&key);
                    }
                }
            }
            result
        }

        /// Inserts a child widget position `index`
        ///
        /// Panics if `index > len`.
        ///
        /// The new child is configured immediately. Triggers [`TkAction::RESIZE`].
        pub fn insert(&mut self, mgr: &mut SetRectMgr, index: usize, widget: W) {
            for v in self.id_map.values_mut() {
                if *v >= index {
                    *v += 1;
                }
            }
            self.widgets.insert(index, widget);
            let id = self.make_child_id(index);
            mgr.configure(id, &mut self.widgets[index]);
            *mgr |= TkAction::RESIZE;
        }

        /// Removes the child widget at position `index`
        ///
        /// Panics if `index` is out of bounds.
        ///
        /// Triggers [`TkAction::RESIZE`].
        pub fn remove(&mut self, mgr: &mut SetRectMgr, index: usize) -> W {
            let w = self.widgets.remove(index);
            if w.id_ref().is_valid() {
                if let Some(key) = w.id_ref().next_key_after(self.id_ref()) {
                    self.id_map.remove(&key);
                }
            }

            *mgr |= TkAction::RESIZE;

            for v in self.id_map.values_mut() {
                if *v > index {
                    *v -= 1;
                }
            }
            w
        }

        /// Replace the child at `index`
        ///
        /// Panics if `index` is out of bounds.
        ///
        /// The new child is configured immediately. Triggers [`TkAction::RESIZE`].
        pub fn replace(&mut self, mgr: &mut SetRectMgr, index: usize, mut w: W) -> W {
            std::mem::swap(&mut w, &mut self.widgets[index]);

            if w.id_ref().is_valid() {
                if let Some(key) = w.id_ref().next_key_after(self.id_ref()) {
                    self.id_map.remove(&key);
                }
            }

            let id = self.make_child_id(index);
            mgr.configure(id, &mut self.widgets[index]);

            *mgr |= TkAction::RESIZE;

            w
        }

        /// Append child widgets from an iterator
        ///
        /// New children are configured immediately. Triggers [`TkAction::RESIZE`].
        pub fn extend<T: IntoIterator<Item = W>>(&mut self, mgr: &mut SetRectMgr, iter: T) {
            let old_len = self.widgets.len();
            self.widgets.extend(iter);
            for index in old_len..self.widgets.len() {
                let id = self.make_child_id(index);
                mgr.configure(id, &mut self.widgets[index]);
            }

            *mgr |= TkAction::RESIZE;
        }

        /// Resize, using the given closure to construct new widgets
        ///
        /// New children are configured immediately. Triggers [`TkAction::RESIZE`].
        pub fn resize_with<F: Fn(usize) -> W>(&mut self, mgr: &mut SetRectMgr, len: usize, f: F) {
            let old_len = self.widgets.len();

            if len < old_len {
                *mgr |= TkAction::RESIZE;
                loop {
                    let w = self.widgets.pop().unwrap();
                    if w.id_ref().is_valid() {
                        if let Some(key) = w.id_ref().next_key_after(self.id_ref()) {
                            self.id_map.remove(&key);
                        }
                    }
                    if len == self.widgets.len() {
                        return;
                    }
                }
            }

            if len > old_len {
                self.widgets.reserve(len - old_len);
                for index in old_len..len {
                    let id = self.make_child_id(index);
                    let mut widget = f(index);
                    mgr.configure(id, &mut widget);
                    self.widgets.push(widget);
                }
                *mgr |= TkAction::RESIZE;
            }
        }

        /// Iterate over childern
        pub fn iter(&self) -> impl Iterator<Item = &W> {
            ListIter {
                list: &self.widgets,
            }
        }

        /// Mutably iterate over childern
        pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut W> {
            ListIterMut {
                list: &mut self.widgets,
            }
        }
    }

    impl Index<usize> for Self {
        type Output = W;

        fn index(&self, index: usize) -> &Self::Output {
            &self.widgets[index]
        }
    }

    impl IndexMut<usize> for Self {
        fn index_mut(&mut self, index: usize) -> &mut Self::Output {
            &mut self.widgets[index]
        }
    }
}

impl<D: Directional + Default, W: Widget> FromIterator<W> for List<D, W> {
    #[inline]
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = W>,
    {
        Self::new_vec(iter.into_iter().collect())
    }
}

struct ListIter<'a, W: Widget> {
    list: &'a [W],
}
impl<'a, W: Widget> Iterator for ListIter<'a, W> {
    type Item = &'a W;
    fn next(&mut self) -> Option<Self::Item> {
        if let Some((first, rest)) = self.list.split_first() {
            self.list = rest;
            Some(first)
        } else {
            None
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.len();
        (len, Some(len))
    }
}
impl<'a, W: Widget> ExactSizeIterator for ListIter<'a, W> {
    fn len(&self) -> usize {
        self.list.len()
    }
}

struct ListIterMut<'a, W: Widget> {
    list: &'a mut [W],
}
impl<'a, W: Widget> Iterator for ListIterMut<'a, W> {
    type Item = &'a mut W;
    fn next(&mut self) -> Option<Self::Item> {
        let list = std::mem::take(&mut self.list);
        if let Some((first, rest)) = list.split_first_mut() {
            self.list = rest;
            Some(first)
        } else {
            None
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.len();
        (len, Some(len))
    }
}
impl<'a, W: Widget> ExactSizeIterator for ListIterMut<'a, W> {
    fn len(&self) -> usize {
        self.list.len()
    }
}
