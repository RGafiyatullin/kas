// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Counter example (simple button)

use kas::class::HasString;
use kas::event::{Manager, VoidMsg, VoidResponse};
use kas::macros::{make_widget, VoidMsg};
use kas::widget::{Label, TextButton, Window};

#[derive(Clone, Debug, VoidMsg)]
enum Message {
    Decr,
    Incr,
}

fn main() -> Result<(), kas_wgpu::Error> {
    env_logger::init();

    let buttons = make_widget! {
        #[layout(row)]
        #[handler(msg = Message)]
        struct {
            #[widget] _ = TextButton::new_msg("−", Message::Decr),
            #[widget] _ = TextButton::new_msg("+", Message::Incr),
        }
    };
    let window = Window::new(
        "Counter",
        make_widget! {
            #[layout(column)]
            #[handler(msg = VoidMsg)]
            struct {
                #[widget(halign=centre)] display: Label<String> = Label::from("0"),
                #[widget(handler = handle_button)] buttons -> Message = buttons,
                counter: usize = 0,
            }
            impl {
                fn handle_button(&mut self, mgr: &mut Manager, msg: Message)
                    -> VoidResponse
                {
                    match msg {
                        Message::Decr => {
                            self.counter = self.counter.saturating_sub(1);
                            *mgr |= self.display.set_string(self.counter.to_string());
                        }
                        Message::Incr => {
                            self.counter = self.counter.saturating_add(1);
                            *mgr |= self.display.set_string(self.counter.to_string());
                        }
                    };
                    VoidResponse::None
                }
            }
        },
    );

    let theme = kas_theme::ShadedTheme::new().with_font_size(24.0);
    kas_wgpu::Toolkit::new(theme)?.with(window)?.run()
}
