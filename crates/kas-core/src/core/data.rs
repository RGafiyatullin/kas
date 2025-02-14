// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Widget data types

#[allow(unused)]
use super::Layout;
use super::{Widget, WidgetId};
use crate::dir::Direction;
use crate::event::EventMgr;
use crate::geom::Rect;

#[cfg(feature = "winit")]
pub use winit::window::Icon;

/// An icon used for the window titlebar, taskbar, etc.
#[cfg(not(feature = "winit"))]
#[derive(Clone)]
pub struct Icon;
#[cfg(not(feature = "winit"))]
impl Icon {
    /// Creates an `Icon` from 32bpp RGBA data.
    ///
    /// The length of `rgba` must be divisible by 4, and `width * height` must equal
    /// `rgba.len() / 4`. Otherwise, this will return a `BadIcon` error.
    pub fn from_rgba(
        rgba: Vec<u8>,
        width: u32,
        height: u32,
    ) -> Result<Self, impl std::error::Error> {
        let _ = (rgba, width, height);
        Result::<Self, std::convert::Infallible>::Ok(Icon)
    }
}

/// Common widget data
///
/// This type may be used for a [`Widget`]'s `core: widget_core!()` field.
#[derive(Default, Debug)]
pub struct CoreData {
    pub rect: Rect,
    pub id: WidgetId,
}

/// Note: the clone has default-initialised identifier.
/// Configuration and layout solving is required as for any other widget.
impl Clone for CoreData {
    fn clone(&self) -> Self {
        CoreData {
            rect: self.rect,
            id: WidgetId::default(),
        }
    }
}

/// A widget which escapes its parent's rect
///
/// A pop-up is a special widget drawn either as a layer over the existing
/// window or in a new borderless window. It should be precisely positioned
/// *next to* it's `parent`'s `rect`, in the specified `direction` (or, if not
/// possible, in the opposite direction).
///
/// A pop-up is in some ways an ordinary child widget and in some ways not.
/// The pop-up widget should be a permanent child of its parent, but is not
/// visible until [`EventMgr::add_popup`] is called.
///
/// A pop-up widget's rect is not contained by its parent, therefore the parent
/// must not call any [`Layout`] methods on the pop-up (whether or not it is
/// visible). The window is responsible for calling these methods.
//
// NOTE: it's tempting to include a pointer to the widget here. There are two
// options: (a) an unsafe aliased pointer or (b) Rc<RefCell<dyn Widget>>.
// Option (a) should work but is an unnecessary performance hack; (b) could in
// theory work but requires adjusting WidgetChildren::get, find etc. to take a
// closure instead of returning a reference, causing *significant* complication.
#[derive(Clone, Debug)]
pub struct Popup {
    pub id: WidgetId,
    pub parent: WidgetId,
    pub direction: Direction,
}

/// Functionality required by a window
pub trait Window: Widget {
    /// Get the window title
    fn title(&self) -> &str;

    /// Get the window icon, if any
    ///
    /// Default: `None`
    #[inline]
    fn icon(&self) -> Option<Icon> {
        None
    }

    /// Whether to limit the maximum size of a window
    ///
    /// All widgets' size rules allow calculation of two sizes: the minimum
    /// size and the ideal size. Windows are initially sized to the ideal size.
    /// This option controls whether the window size is restricted by the
    /// calculated minimum size and by the ideal size.
    ///
    /// Return value is `(restrict_min, restrict_max)`. Suggested is to use
    /// `(true, true)` for simple dialog boxes and `(true, false)` for complex
    /// windows.
    ///
    /// Default: `(true, false)`
    #[inline]
    fn restrict_dimensions(&self) -> (bool, bool) {
        (true, false)
    }

    /// Handle closure of self
    ///
    /// This allows for actions on destruction.
    ///
    /// Default: do nothing.
    fn handle_closure(&mut self, mgr: &mut EventMgr) {
        let _ = mgr;
    }
}
