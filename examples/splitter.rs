// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Counter example (simple button)

use kas::event::EventMgr;
use kas::macros::impl_singleton;
use kas::widgets::{EditField, RowSplitter, TextButton};
use kas::{Widget, Window};

#[derive(Clone, Debug)]
enum Message {
    Decr,
    Incr,
}

fn main() -> kas::shell::Result<()> {
    env_logger::init();

    let panes = (0..2).map(|n| EditField::new(format!("Pane {}", n + 1)).multi_line(true));
    let panes = RowSplitter::<EditField>::new(panes.collect());

    let window = impl_singleton! {
        #[widget{
            layout = column: [
                row: [
                    TextButton::new_msg("−", Message::Decr),
                    TextButton::new_msg("+", Message::Incr),
                ],
                self.panes,
            ];
        }]
        #[derive(Debug)]
        struct {
            core: widget_core!(),
            #[widget] panes: RowSplitter<EditField> = panes,
        }
        impl Widget for Self {
            fn handle_message(&mut self, mgr: &mut EventMgr, _: usize) {
                if let Some(msg) = mgr.try_pop_msg::<Message>() {
                    match msg {
                        Message::Decr => {
                            mgr.set_rect_mgr(|mgr| self.panes.pop(mgr));
                        }
                        Message::Incr => {
                            let n = self.panes.len() + 1;
                            mgr.set_rect_mgr(|mgr| self.panes.push(
                                mgr,
                                EditField::new(format!("Pane {}", n)).multi_line(true)
                            ));
                        }
                    };
                }
            }
        }
        impl Window for Self {
            fn title(&self) -> &str { "Slitter panes" }
        }
    };

    let theme = kas::theme::ShadedTheme::new();
    kas::shell::Toolkit::new(theme)?.with(window)?.run()
}
