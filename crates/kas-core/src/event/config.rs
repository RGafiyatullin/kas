// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event handling configuration

mod shortcuts;
pub use shortcuts::Shortcuts;

use super::ModifiersState;
use crate::cast::{Cast, CastFloat};
use crate::geom::Offset;
#[cfg(feature = "config")]
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

/// Event handling configuration
///
/// This is serializable (using `feature = "config"`) with the following fields:
///
/// > `menu_delay_ms`: `u32` (milliseconds) \
/// > `touch_select_delay_ms`: `u32` (milliseconds) \
/// > `scroll_flick_timeout_ms`: `u32` (milliseconds) \
/// > `scroll_flick_mul`: `f32` (unitless, applied each second) \
/// > `scroll_flick_sub`: `f32` (pixels per second) \
/// > `pan_dist_thresh`: `f32` (pixels) \
/// > `mouse_pan`: [`MousePan`] \
/// > `mouse_text_pan`: [`MousePan`] \
/// > `mouse_nav_focus`: `bool` \
/// > `touch_nav_focus`: `bool` \
/// > `shortcuts`: [`Shortcuts`]
///
/// For descriptions of configuration effects, see [`WindowConfig`] methods.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "config", derive(Serialize, Deserialize))]
pub struct Config {
    #[cfg_attr(feature = "config", serde(default = "defaults::menu_delay_ms"))]
    pub menu_delay_ms: u32,

    #[cfg_attr(feature = "config", serde(default = "defaults::touch_select_delay_ms"))]
    pub touch_select_delay_ms: u32,

    #[cfg_attr(feature = "config", serde(default = "defaults::scroll_lines"))]
    pub scroll_lines: f32,

    #[cfg_attr(
        feature = "config",
        serde(default = "defaults::scroll_flick_timeout_ms")
    )]
    pub scroll_flick_timeout_ms: u32,

    #[cfg_attr(feature = "config", serde(default = "defaults::scroll_flick_mul"))]
    pub scroll_flick_mul: f32,

    #[cfg_attr(feature = "config", serde(default = "defaults::scroll_flick_sub"))]
    pub scroll_flick_sub: f32,

    #[cfg_attr(feature = "config", serde(default = "defaults::pan_dist_thresh"))]
    pub pan_dist_thresh: f32,

    #[cfg_attr(feature = "config", serde(default = "defaults::mouse_pan"))]
    pub mouse_pan: MousePan,
    #[cfg_attr(feature = "config", serde(default = "defaults::mouse_text_pan"))]
    pub mouse_text_pan: MousePan,

    #[cfg_attr(feature = "config", serde(default = "defaults::mouse_nav_focus"))]
    pub mouse_nav_focus: bool,
    #[cfg_attr(feature = "config", serde(default = "defaults::touch_nav_focus"))]
    pub touch_nav_focus: bool,

    #[cfg_attr(feature = "config", serde(default = "Shortcuts::platform_defaults"))]
    pub shortcuts: Shortcuts,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            menu_delay_ms: defaults::menu_delay_ms(),
            touch_select_delay_ms: defaults::touch_select_delay_ms(),
            scroll_lines: defaults::scroll_lines(),
            scroll_flick_timeout_ms: defaults::scroll_flick_timeout_ms(),
            scroll_flick_mul: defaults::scroll_flick_mul(),
            scroll_flick_sub: defaults::scroll_flick_sub(),
            pan_dist_thresh: defaults::pan_dist_thresh(),
            mouse_pan: defaults::mouse_pan(),
            mouse_text_pan: defaults::mouse_text_pan(),
            mouse_nav_focus: defaults::mouse_nav_focus(),
            touch_nav_focus: defaults::touch_nav_focus(),
            shortcuts: Shortcuts::platform_defaults(),
        }
    }
}

/// Wrapper around [`Config`] to handle window-specific scaling
#[derive(Clone, Debug)]
pub struct WindowConfig {
    config: Rc<RefCell<Config>>,
    scroll_dist: f32,
    scroll_flick_sub: f32,
    pan_dist_thresh: f32,
}

impl WindowConfig {
    /// Construct
    #[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
    #[cfg_attr(doc_cfg, doc(cfg(internal_doc)))]
    pub fn new(config: Rc<RefCell<Config>>, scale_factor: f32) -> Self {
        let mut w = WindowConfig {
            config,
            scroll_dist: f32::NAN,
            scroll_flick_sub: f32::NAN,
            pan_dist_thresh: f32::NAN,
        };
        w.set_scale_factor(scale_factor);
        w
    }

    /// Set scale factor
    #[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
    #[cfg_attr(doc_cfg, doc(cfg(internal_doc)))]
    pub fn set_scale_factor(&mut self, scale_factor: f32) {
        let base = self.config.borrow();
        const LINE_HEIGHT: f32 = 19.0; // TODO: maybe we shouldn't assume this?
        self.scroll_dist = base.scroll_lines * LINE_HEIGHT;
        self.scroll_flick_sub = base.scroll_flick_sub * scale_factor;
        self.pan_dist_thresh = base.pan_dist_thresh * scale_factor;
    }

    /// Delay before opening/closing menus on mouse hover
    #[inline]
    pub fn menu_delay(&self) -> Duration {
        Duration::from_millis(self.config.borrow().menu_delay_ms.cast())
    }

    /// Delay before switching from panning to (text) selection mode
    #[inline]
    pub fn touch_select_delay(&self) -> Duration {
        Duration::from_millis(self.config.borrow().touch_select_delay_ms.cast())
    }

    /// Get distance in pixels to scroll due to mouse wheel
    ///
    /// Calculates scroll distance from `(horiz, vert)` lines.
    ///
    /// If `line_height` is provided, scroll distance is based on this value,
    /// otherwise it is based on an arbitrary line height.
    pub fn scroll_distance(&self, lines: (f32, f32), line_height: Option<f32>) -> Offset {
        let dist = match line_height {
            Some(height) => height * self.config.borrow().scroll_lines,
            None => self.scroll_dist,
        };
        Offset(
            (dist * lines.0).cast_nearest(),
            (dist * lines.1).cast_nearest(),
        )
    }

    /// Controls activation of glide/momentum scrolling
    ///
    /// This is the maximum time between the last press-movement and final
    /// release to activate momentum scrolling mode. The last few `PressMove`
    /// events within this time window are used to calculate the initial speed.
    #[inline]
    pub fn scroll_flick_timeout(&self) -> Duration {
        Duration::from_millis(self.config.borrow().scroll_flick_timeout_ms.cast())
    }

    /// Scroll flick velocity decay: `(mul, sub)`
    ///
    /// The `mul` factor describes exponential decay: effectively, velocity is
    /// multiplied by `mul` every second. This is the dominant decay factor at
    /// high speeds; `mul = 1.0` implies no decay while `mul = 0.0` implies an
    /// instant stop.
    ///
    /// The `sub` factor describes linear decay: effectively, speed is reduced
    /// by `sub` every second. This is the dominant decay factor at low speeds.
    /// Units are pixels/second (output is adjusted for the window's scale factor).
    #[inline]
    pub fn scroll_flick_decay(&self) -> (f32, f32) {
        (self.config.borrow().scroll_flick_mul, self.scroll_flick_sub)
    }

    /// Drag distance threshold before panning (scrolling) starts
    ///
    /// When the distance moved is greater than this threshold, panning should
    /// start; otherwise the system should wait for the text-selection timer.
    /// We currently recommend the L-inf distance metric (max of abs of values).
    ///
    /// Units are pixels (output is adjusted for the window's scale factor).
    #[inline]
    pub fn pan_dist_thresh(&self) -> f32 {
        self.pan_dist_thresh
    }

    /// When to pan general widgets (unhandled events) with the mouse
    #[inline]
    pub fn mouse_pan(&self) -> MousePan {
        self.config.borrow().mouse_pan
    }

    /// When to pan text fields with the mouse
    #[inline]
    pub fn mouse_text_pan(&self) -> MousePan {
        self.config.borrow().mouse_text_pan
    }

    /// Whether mouse clicks set keyboard navigation focus
    #[inline]
    pub fn mouse_nav_focus(&self) -> bool {
        self.config.borrow().mouse_nav_focus
    }

    /// Whether touchscreen events set keyboard navigation focus
    #[inline]
    pub fn touch_nav_focus(&self) -> bool {
        self.config.borrow().touch_nav_focus
    }

    /// Access shortcut config
    pub fn shortcuts<F: FnOnce(&Shortcuts) -> T, T>(&self, f: F) -> T {
        let base = self.config.borrow();
        f(&base.shortcuts)
    }
}

/// Other functions
impl Config {
    /// Has the config ever been updated?
    #[inline]
    pub fn is_dirty(&self) -> bool {
        false // current code never updates config
    }
}

/// When mouse-panning is enabled (click+drag to scroll)
///
/// For *text* objects, this may conflict with text selection, hence it is
/// recommended to require a modifier or disable this feature.
///
/// For non-text cases, this does not conflict with other event handlers since
/// panning is only possible when events are otherwise unused, thus `Always` is
/// acceptable (equivalent to touch scrolling).
#[derive(Clone, Copy, Debug, Hash, PartialEq)]
#[cfg_attr(feature = "config", derive(Serialize, Deserialize))]
#[derive(num_enum::IntoPrimitive, num_enum::TryFromPrimitive)]
#[repr(u8)]
pub enum MousePan {
    /// Disable
    Never,
    /// Only enable when the Alt key is held
    WithAlt,
    /// Only enable when the Ctrl key is held
    WithCtrl,
    /// Always enabled
    Always,
}

impl MousePan {
    /// Is this enabled with the current modifiers?
    pub fn is_enabled_with(self, modifiers: ModifiersState) -> bool {
        match self {
            MousePan::Never => false,
            MousePan::WithAlt => modifiers.alt(),
            MousePan::WithCtrl => modifiers.ctrl(),
            MousePan::Always => true,
        }
    }
}

mod defaults {
    use super::MousePan;

    pub fn menu_delay_ms() -> u32 {
        250
    }
    pub fn touch_select_delay_ms() -> u32 {
        1000
    }
    pub fn scroll_lines() -> f32 {
        3.0
    }
    pub fn scroll_flick_timeout_ms() -> u32 {
        25
    }
    pub fn scroll_flick_mul() -> f32 {
        0.5
    }
    pub fn scroll_flick_sub() -> f32 {
        100.0
    }
    pub fn pan_dist_thresh() -> f32 {
        2.1
    }
    pub fn mouse_pan() -> MousePan {
        MousePan::Always
    }
    pub fn mouse_text_pan() -> MousePan {
        #[cfg(target_os = "windows")]
        {
            MousePan::WithAlt
        }
        #[cfg(not(target_os = "windows"))]
        {
            MousePan::WithCtrl
        }
    }
    pub fn mouse_nav_focus() -> bool {
        true
    }
    pub fn touch_nav_focus() -> bool {
        true
    }
}
