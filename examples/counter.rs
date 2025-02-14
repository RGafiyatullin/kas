// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Counter example (simple button)

use kas::prelude::*;
use kas::widgets::{Label, TextButton};

fn main() -> kas::shell::Result<()> {
    env_logger::init();

    #[derive(Clone, Debug)]
    struct Increment(i32);

    impl_scope! {
        #[widget{
            layout = column: [
                align(center): self.display,
                row: [
                    TextButton::new_msg("−", Increment(-1)),
                    TextButton::new_msg("+", Increment(1)),
                ],
            ];
        }]
        #[derive(Debug)]
        struct Counter {
            core: widget_core!(),
            #[widget]
            display: Label<String>,
            count: i32,
        }
        impl Self {
            fn new(count: i32) -> Self {
                Counter {
                    core: Default::default(),
                    display: Label::from(count.to_string()),
                    count,
                }
            }
        }
        impl Widget for Self {
            fn handle_message(&mut self, mgr: &mut EventMgr, _: usize) {
                if let Some(Increment(incr)) = mgr.try_pop_msg() {
                    self.count += incr;
                    *mgr |= self.display.set_string(self.count.to_string());
                }
            }
        }
        impl Window for Self {
            fn title(&self) -> &str { "Counter" }
        }
    };

    let theme = kas::theme::ShadedTheme::new().with_font_size(24.0);
    kas::shell::Toolkit::new(theme)?
        .with(Counter::new(0))?
        .run()
}
