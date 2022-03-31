// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Markdown parsing demo

use kas::class::HasStr;
use kas::event::{EventMgr, VoidMsg};
use kas::macros::make_widget;
use kas::text::format::Markdown;
use kas::widgets::{EditBox, EditField, EditGuard, Label, ScrollBarRegion, Window};

#[derive(Debug)]
struct Guard;
impl EditGuard for Guard {
    type Msg = Markdown;

    fn edit(edit: &mut EditField<Self>, _: &mut EventMgr) -> Option<Markdown> {
        let result = Markdown::new(edit.get_str());
        edit.set_error_state(result.is_err());
        Some(match result {
            Ok(md) => md,
            Err(err) => {
                let string = format!("```\n{}\n```", err);
                Markdown::new(&string).unwrap()
            }
        })
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let theme = kas::theme::FlatTheme::new();
    let shell = kas::shell::Toolkit::new(theme)?;

    let doc = r"Markdown document
================

Markdown supports *italic* and **bold** highlighting, ***both***, even with*in* w**o**rds.
As an extension, it also supports ~~strikethrough~~.

Inline `code = 2;` is supported. Code blocks are supported:
```
let x = 1;
let y = x + 1;
```

Markdown supports explicit line breaks —  
via two trailing spaces.  
It also supports lists:

1.  First item
2.  Second item

-   Unenumerated item
-   Another item
";

    let window = Window::new(
        "Markdown parser",
        make_widget! {
            #[widget{
                layout = row: *;
                msg = VoidMsg;
            }]
            struct {
                #[widget(use_msg=update)] editor: EditBox<Guard> =
                    EditBox::new(doc).multi_line(true).with_guard(Guard),
                #[widget] label: ScrollBarRegion<Label<Markdown>> =
                    ScrollBarRegion::new(Label::new(Markdown::new(doc)?)),
            }
            impl Self {
                fn update(&mut self, mgr: &mut EventMgr, md: Markdown) {
                    *mgr |= self.label.set_text(md);
                }
            }
        },
    );

    shell.with(window)?.run()
}
