// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! KAS prelude
//!
//! This module allows convenient importation of common unabiguous items:
//! ```
//! use kas::prelude::*;
//! ```
//!
//! This prelude may be more useful when implementing widgets than when simply
//! using widgets in a GUI.

pub use kas::draw::{DrawHandle, SizeHandle};
pub use kas::event::{Event, Handler, Manager, ManagerState, Response, SendEvent, VoidMsg};
pub use kas::geom::{Coord, Rect, Size};
pub use kas::layout::{AxisInfo, Margins, SizeRules, StretchPolicy};
pub use kas::macros::*;
pub use kas::string::{AccelString, CowString, CowStringL, LabelString};
pub use kas::{class, draw, event, geom, layout, widget};
pub use kas::{Align, AlignHints, Direction, Directional, WidgetId};
pub use kas::{Boxed, TkAction, TkWindow};
pub use kas::{CloneTo, Layout, ThemeApi, Widget, WidgetChildren, WidgetConfig, WidgetCore};
pub use kas::{CoreData, LayoutData};
