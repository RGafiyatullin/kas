// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Message Map widget

use crate::menu;
use kas::prelude::*;
use std::rc::Rc;

impl_scope! {
    /// Wrapper to map messages from the inner widget
    #[autoimpl(Debug ignore self.map)]
    #[autoimpl(Deref, DerefMut using self.inner)]
    #[autoimpl(class_traits using self.inner where W: trait)]
    #[derive(Clone)]
    #[widget{
        layout = single;
        msg = M;
    }]
    pub struct MapResponse<W: Widget, M: 'static> {
        #[widget_core]
        core: kas::CoreData,
        #[widget]
        inner: W,
        map: Rc<dyn Fn(&mut EventMgr, W::Msg) -> Response<M>>,
    }

    impl Self {
        /// Construct
        ///
        /// Any response from the child widget with a message payload is mapped
        /// through the closure `f`.
        pub fn new<F: Fn(&mut EventMgr, W::Msg) -> Response<M> + 'static>(child: W, f: F) -> Self {
            Self::new_rc(child, Rc::new(f))
        }

        /// Construct with an Rc-wrapped method
        ///
        /// Any response from the child widget with a message payload is mapped
        /// through the closure `f`.
        pub fn new_rc(child: W, f: Rc<dyn Fn(&mut EventMgr, W::Msg) -> Response<M>>) -> Self {
            MapResponse {
                core: Default::default(),
                inner: child,
                map: f,
            }
        }
    }

    impl SendEvent for Self {
        fn send(&mut self, mgr: &mut EventMgr, id: WidgetId, event: Event) -> Response<Self::Msg> {
            if self.eq_id(&id) {
                self.handle(mgr, event)
            } else {
                let r = self.inner.send(mgr, id.clone(), event);
                r.try_into().unwrap_or_else(|msg| {
                    (self.map)(mgr, msg)
                })
            }
        }
    }

    impl<W: menu::Menu, M: 'static> menu::Menu for MapResponse<W, M> {
        fn sub_items(&mut self) -> Option<menu::SubItems> {
            self.inner.sub_items()
        }
        fn menu_is_open(&self) -> bool {
            self.inner.menu_is_open()
        }
        fn set_menu_path(&mut self, mgr: &mut EventMgr, target: Option<&WidgetId>, set_focus: bool) {
            self.inner.set_menu_path(mgr, target, set_focus);
        }
    }
}
