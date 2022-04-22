// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Data list example (indirect representation)
//!
//! This is a variant of `data-list` using the [`ListView`] widget to create a
//! dynamic view over a lazy, indirect data structure. Maximum data length is
//! thus only limited by the data types used (specifically the `i32` type used
//! to calculate the maximum scroll offset).

use kas::prelude::*;
use kas::updatable::*;
use kas::widgets::view::{Driver, ListView};
use kas::widgets::*;
use std::cell::RefCell;
use std::collections::HashMap;

#[derive(Clone, Debug)]
enum Control {
    Set(usize),
    Dir,
    Update(String),
}

#[derive(Clone, Debug)]
enum Button {
    Decr,
    Incr,
    Set,
}

#[derive(Clone, Debug)]
enum EntryMsg {
    Select,
    Update(String),
}

#[derive(Debug)]
struct MyData {
    ver: u64,
    len: usize,
    active: usize,
    strings: HashMap<usize, String>,
}
impl MyData {
    fn new(len: usize) -> Self {
        MyData {
            ver: 1,
            len,
            active: 0,
            strings: HashMap::new(),
        }
    }
    fn get(&self, index: usize) -> String {
        self.strings
            .get(&index)
            .cloned()
            .unwrap_or_else(|| format!("Entry #{}", index + 1))
    }
}

#[derive(Debug)]
struct MySharedData {
    data: RefCell<MyData>,
    handle: UpdateHandle,
}
impl MySharedData {
    fn new(len: usize) -> Self {
        MySharedData {
            data: RefCell::new(MyData::new(len)),
            handle: UpdateHandle::new(),
        }
    }
    fn set_len(&mut self, len: usize) -> (Option<String>, UpdateHandle) {
        let mut new_text = None;
        let mut data = self.data.borrow_mut();
        data.ver += 1;
        data.len = len;
        if data.active >= len && len > 0 {
            data.active = len - 1;
            new_text = Some(data.get(data.active));
        }
        (new_text, self.handle)
    }
}
impl ListData for MySharedData {
    type Key = usize;
    type Item = (usize, bool, String);

    fn update_handles(&self) -> Vec<UpdateHandle> {
        vec![self.handle]
    }
    fn version(&self) -> u64 {
        self.data.borrow().ver
    }

    fn len(&self) -> usize {
        self.data.borrow().len
    }
    fn make_id(&self, parent: &WidgetId, key: &Self::Key) -> WidgetId {
        parent.make_child(*key)
    }
    fn reconstruct_key(&self, parent: &WidgetId, child: &WidgetId) -> Option<Self::Key> {
        child.next_key_after(parent)
    }

    fn contains_key(&self, key: &Self::Key) -> bool {
        *key < self.len()
    }

    fn get_cloned(&self, key: &Self::Key) -> Option<Self::Item> {
        let index = *key;
        let data = self.data.borrow();
        let is_active = data.active == index;
        let text = data.get(index);
        Some((index, is_active, text))
    }

    fn update(&self, _: &Self::Key, _: Self::Item) -> Option<UpdateHandle> {
        unimplemented!()
    }

    fn on_message(&self, mgr: &mut EventMgr, key: &Self::Key) -> Option<UpdateHandle> {
        mgr.try_pop_msg().map(|msg| {
            let mut data = self.data.borrow_mut();
            data.ver += 1;
            match msg {
                EntryMsg::Select => {
                    data.active = *key;
                }
                EntryMsg::Update(text) => {
                    data.strings.insert(*key, text.clone());
                }
            }
            mgr.push_msg(Control::Update(data.get(data.active)));
            self.handle
        })
    }

    fn iter_vec(&self, limit: usize) -> Vec<Self::Key> {
        (0..limit.min(self.len())).collect()
    }

    fn iter_vec_from(&self, start: usize, limit: usize) -> Vec<Self::Key> {
        let len = self.len();
        (start.min(len)..(start + limit).min(len)).collect()
    }
}

// TODO: it would be nicer to use EditBox::new(..).on_edit(..), but that produces
// an object with unnamable type, which is a problem.
#[derive(Clone, Debug)]
struct ListEntryGuard;
impl EditGuard for ListEntryGuard {
    fn edit(entry: &mut EditField<Self>, mgr: &mut EventMgr) {
        mgr.push_msg(EntryMsg::Update(entry.get_string()));
    }
}

impl_scope! {
    // The list entry
    #[derive(Clone, Debug)]
    #[widget{
        layout = column: *;
    }]
    struct ListEntry {
        #[widget_core]
        core: CoreData,
        #[widget]
        label: StringLabel,
        #[widget]
        radio: RadioBox,
        #[widget]
        entry: EditBox<ListEntryGuard>,
    }
}

#[derive(Debug)]
struct MyDriver {
    radio_group: RadioBoxGroup,
}
impl Driver<(usize, bool, String)> for MyDriver {
    type Widget = ListEntry;

    fn make(&self) -> Self::Widget {
        // Default instances are not shown, so the data is unimportant
        ListEntry {
            core: Default::default(),
            label: Label::new(String::default()),
            radio: RadioBox::new("display this entry", self.radio_group.clone())
                .on_select(|mgr| mgr.push_msg(EntryMsg::Select)),
            entry: EditBox::new(String::default()).with_guard(ListEntryGuard),
        }
    }
    fn set(&self, widget: &mut Self::Widget, data: (usize, bool, String)) -> TkAction {
        let label = format!("Entry number {}", data.0 + 1);
        widget.label.set_string(label)
            | widget.radio.set_bool(data.1)
            | widget.entry.set_string(data.2)
    }
    fn get(&self, _widget: &Self::Widget) -> Option<(usize, bool, String)> {
        None // unused
    }
}

fn main() -> kas::shell::Result<()> {
    env_logger::init();

    let controls = make_widget! {
        #[widget{
            layout = row: *;
        }]
        struct {
            #[widget] _ = Label::new("Number of rows:"),
            #[widget] edit: impl HasString = EditBox::new("3")
                .on_afl(|text, _| text.parse::<usize>().ok()),
            #[widget] _ = TextButton::new_msg("Set", Button::Set),
            #[widget] _ = TextButton::new_msg("−", Button::Decr),
            #[widget] _ = TextButton::new_msg("+", Button::Incr),
            #[widget] _ = TextButton::new_msg("↓↑", Control::Dir),
            n: usize = 3,
        }
        impl Handler for Self {
            fn on_message(&mut self, mgr: &mut EventMgr, index: usize) {
                if index == widget_index![self.edit] {
                    if let Some(n) = mgr.try_pop_msg::<usize>() {
                        if n != self.n {
                            self.n = n;
                            mgr.push_msg(Control::Set(n))
                        }
                    }
                } else if let Some(msg) = mgr.try_pop_msg::<Button>() {
                    let n = match msg {
                        Button::Decr => self.n.saturating_sub(1),
                        Button::Incr => self.n.saturating_add(1),
                        Button::Set => self.n,
                    };
                    *mgr |= self.edit.set_string(n.to_string());
                    self.n = n;
                    mgr.push_msg(Control::Set(n));
                }
            }
        }
    };

    let driver = MyDriver {
        radio_group: Default::default(),
    };
    let data = MySharedData::new(3);
    type MyList = ListView<Direction, MySharedData, MyDriver>;
    let list = ListView::new_with_dir_driver(Direction::Down, driver, data);

    let window = Window::new(
        "Dynamic widget demo",
        make_widget! {
            #[widget{
                layout = column: *;
            }]
            struct {
                #[widget] _ = Label::new("Demonstration of dynamic widget creation / deletion"),
                #[widget] _ = controls,
                #[widget] _ = Label::new("Contents of selected entry:"),
                #[widget] display: StringLabel = Label::from("Entry #0"),
                #[widget] _ = Separator::new(),
                #[widget] list: ScrollBars<MyList> =
                    ScrollBars::new(list).with_bars(false, true),
            }
            impl Handler for Self {
                fn on_message(&mut self, mgr: &mut EventMgr, _: usize) {
                    if let Some(control) = mgr.try_pop_msg::<Control>() {
                        match control {
                            Control::Set(len) => {
                                let (opt_text, handle) = self.list.data_mut().set_len(len);
                                if let Some(text) = opt_text {
                                    *mgr |= self.display.set_string(text);
                                }
                                mgr.trigger_update(handle, 0);
                            }
                            Control::Dir => {
                                let dir = self.list.direction().reversed();
                                *mgr |= self.list.set_direction(dir);
                            }
                            Control::Update(text) => {
                                *mgr |= self.display.set_string(text);
                            }
                        }
                    }
                }
            }
        },
    );

    let theme = kas::theme::ShadedTheme::new();
    kas::shell::Toolkit::new(theme)?.with(window)?.run()
}
