// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event manager

// Without winit, several things go unused
#![cfg_attr(not(feature = "winit"), allow(unused))]

use linear_map::{set::LinearSet, LinearMap};
use log::trace;
use smallvec::SmallVec;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::time::Instant;
use std::u16;

use super::*;
use crate::cast::Cast;
use crate::geom::{Coord, Offset};
#[allow(unused)]
use crate::WidgetConfig; // for doc-links
use crate::{ShellWindow, TkAction, Widget, WidgetId, WindowId};

mod mgr_pub;
mod mgr_shell;

/// Controls the types of events delivered by [`EventMgr::request_grab`]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GrabMode {
    /// Deliver [`Event::PressMove`] and [`Event::PressEnd`] for each press
    Grab,
    /// Deliver [`Event::Pan`] events, with scaling and rotation
    PanFull,
    /// Deliver [`Event::Pan`] events, with scaling
    PanScale,
    /// Deliver [`Event::Pan`] events, with rotation
    PanRotate,
    /// Deliver [`Event::Pan`] events, without scaling or rotation
    PanOnly,
}

#[derive(Clone, Debug)]
struct MouseGrab {
    button: MouseButton,
    repetitions: u32,
    start_id: WidgetId,
    cur_id: Option<WidgetId>,
    depress: Option<WidgetId>,
    mode: GrabMode,
    pan_grab: (u16, u16),
    coord: Coord,
    delta: Offset,
}

impl MouseGrab {
    fn flush_move(&mut self) -> Option<(WidgetId, Event)> {
        if self.delta != Offset::ZERO {
            let event = Event::PressMove {
                source: PressSource::Mouse(self.button, self.repetitions),
                cur_id: self.cur_id.clone(),
                coord: self.coord,
                delta: self.delta,
            };
            self.delta = Offset::ZERO;
            Some((self.start_id.clone(), event))
        } else {
            None
        }
    }
}

#[derive(Clone, Debug)]
struct TouchGrab {
    id: u64,
    start_id: WidgetId,
    depress: Option<WidgetId>,
    cur_id: Option<WidgetId>,
    last_move: Coord,
    coord: Coord,
    mode: GrabMode,
    pan_grab: (u16, u16),
}

impl TouchGrab {
    fn flush_move(&mut self) -> Option<(WidgetId, Event)> {
        if self.last_move != self.coord {
            let delta = self.coord - self.last_move;
            let target = self.start_id.clone();
            let event = Event::PressMove {
                source: PressSource::Touch(self.id),
                cur_id: self.cur_id.clone(),
                coord: self.coord,
                delta,
            };
            self.last_move = self.coord;
            Some((target, event))
        } else {
            None
        }
    }
}

const MAX_PAN_GRABS: usize = 2;

#[derive(Clone, Debug)]
struct PanGrab {
    id: WidgetId,
    mode: GrabMode,
    source_is_touch: bool,
    n: u16,
    coords: [(Coord, Coord); MAX_PAN_GRABS],
}

#[derive(Clone, Debug)]
#[allow(clippy::enum_variant_names)] // they all happen to be about Focus
enum Pending {
    LostCharFocus(WidgetId),
    LostSelFocus(WidgetId),
    SetNavFocus(WidgetId, bool),
}

/// Event manager state
///
/// This struct encapsulates window-specific event-handling state and handling.
/// Most operations are only available via a [`EventMgr`] handle, though some
/// are available on this struct.
///
/// Besides event handling, this struct also configures widgets.
///
/// Some methods are intended only for usage by KAS shells and are hidden from
/// documentation unless the `internal_doc` feature is enabled. Only [winit]
/// events are currently supported; changes will be required to generalise this.
///
/// [winit]: https://github.com/rust-windowing/winit
//
// Note that the most frequent usage of fields is to check highlighting states
// for each widget during drawing. Most fields contain only a few values, hence
// `SmallVec` is used to keep contents in local memory.
#[derive(Debug)]
pub struct EventState {
    config: Rc<RefCell<Config>>,
    scale_factor: f32,
    widget_count: usize,
    modifiers: ModifiersState,
    /// char focus is on same widget as sel_focus; otherwise its value is ignored
    char_focus: bool,
    sel_focus: Option<WidgetId>,
    nav_focus: Option<WidgetId>,
    nav_fallback: Option<WidgetId>,
    hover: Option<WidgetId>,
    hover_icon: CursorIcon,
    key_depress: LinearMap<u32, WidgetId>,
    last_mouse_coord: Coord,
    last_click_button: MouseButton,
    last_click_repetitions: u32,
    last_click_timeout: Instant,
    mouse_grab: Option<MouseGrab>,
    touch_grab: SmallVec<[TouchGrab; 8]>,
    pan_grab: SmallVec<[PanGrab; 4]>,
    accel_stack: Vec<(bool, HashMap<VirtualKeyCode, WidgetId>)>,
    accel_layers: HashMap<WidgetId, (bool, HashMap<VirtualKeyCode, WidgetId>)>,
    // For each: (WindowId of popup, popup descriptor, old nav focus)
    popups: SmallVec<[(WindowId, crate::Popup, Option<WidgetId>); 16]>,
    popup_removed: SmallVec<[(WidgetId, WindowId); 16]>,
    time_updates: Vec<(Instant, WidgetId, u64)>,
    // TODO(opt): consider other containers, e.g. C++ multimap
    // or sorted Vec with binary search yielding a range
    handle_updates: HashMap<UpdateHandle, LinearSet<WidgetId>>,
    pending: SmallVec<[Pending; 8]>,
    #[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
    #[cfg_attr(doc_cfg, doc(cfg(internal_doc)))]
    pub action: TkAction,
}

/// internals
impl EventState {
    #[inline]
    fn char_focus(&self) -> Option<WidgetId> {
        if self.char_focus {
            self.sel_focus.clone()
        } else {
            None
        }
    }

    fn set_pan_on(
        &mut self,
        id: WidgetId,
        mode: GrabMode,
        source_is_touch: bool,
        coord: Coord,
    ) -> (u16, u16) {
        for (gi, grab) in self.pan_grab.iter_mut().enumerate() {
            if grab.id == id {
                if grab.source_is_touch != source_is_touch {
                    self.remove_pan(gi);
                    break;
                }

                let index = grab.n;
                if usize::from(index) < MAX_PAN_GRABS {
                    grab.coords[usize::from(index)] = (coord, coord);
                }
                grab.n = index + 1;
                return (gi.cast(), index);
            }
        }

        let gj = self.pan_grab.len().cast();
        let n = 1;
        let mut coords: [(Coord, Coord); MAX_PAN_GRABS] = Default::default();
        coords[0] = (coord, coord);
        trace!("EventMgr: start pan grab {} on {}", self.pan_grab.len(), id);
        self.pan_grab.push(PanGrab {
            id,
            mode,
            source_is_touch,
            n,
            coords,
        });
        (gj, 0)
    }

    fn remove_pan(&mut self, index: usize) {
        trace!("EventMgr: end pan grab {}", index);
        self.pan_grab.remove(index);
        if let Some(grab) = &mut self.mouse_grab {
            let p0 = grab.pan_grab.0;
            if usize::from(p0) >= index && p0 != u16::MAX {
                grab.pan_grab.0 = p0 - 1;
            }
        }
        for grab in self.touch_grab.iter_mut() {
            let p0 = grab.pan_grab.0;
            if usize::from(p0) >= index && p0 != u16::MAX {
                grab.pan_grab.0 = p0 - 1;
            }
        }
    }

    fn remove_pan_grab(&mut self, g: (u16, u16)) {
        if let Some(grab) = self.pan_grab.get_mut(usize::from(g.0)) {
            grab.n -= 1;
            if grab.n == 0 {
                return self.remove_pan(g.0.into());
            }
            assert!(grab.source_is_touch);
            for i in (usize::from(g.1))..(usize::from(grab.n) - 1) {
                grab.coords[i] = grab.coords[i + 1];
            }
        } else {
            return; // shouldn't happen
        }

        // Note: the fact that grab.n > 0 implies source is a touch event!
        for grab in self.touch_grab.iter_mut() {
            if grab.pan_grab.0 == g.0 && grab.pan_grab.1 > g.1 {
                grab.pan_grab.1 -= 1;
                if usize::from(grab.pan_grab.1) == MAX_PAN_GRABS - 1 {
                    let v = grab.coord;
                    self.pan_grab[usize::from(g.0)].coords[usize::from(grab.pan_grab.1)] = (v, v);
                }
            }
        }
    }
}

/// Manager of event-handling and toolkit actions
///
/// An `EventMgr` is in fact a handle around [`EventState`] and [`ShellWindow`]
/// in order to provide a convenient user-interface during event processing.
///
/// It exposes two interfaces: one aimed at users implementing widgets and UIs
/// and one aimed at shells. The latter is hidden
/// from documentation unless the `internal_doc` feature is enabled.
#[must_use]
pub struct EventMgr<'a> {
    state: &'a mut EventState,
    shell: &'a mut dyn ShellWindow,
    action: TkAction,
}

/// Internal methods
impl<'a> EventMgr<'a> {
    fn set_hover<W: Widget + ?Sized>(&mut self, widget: &W, w_id: Option<WidgetId>) {
        if self.state.hover != w_id {
            trace!("EventMgr: hover = {:?}", w_id);
            if let Some(id) = self.state.hover.take() {
                if widget
                    .find_widget(&id)
                    .map(|w| w.hover_highlight())
                    .unwrap_or(false)
                {
                    self.redraw(id);
                }
            }
            self.state.hover = w_id.clone();

            if let Some(id) = w_id {
                let mut icon = Default::default();
                if let Some(w) = widget.find_widget(&id) {
                    if w.hover_highlight() {
                        self.redraw(id);
                    }
                    icon = w.cursor_icon();
                }
                if icon != self.state.hover_icon {
                    self.state.hover_icon = icon;
                    if self.state.mouse_grab.is_none() {
                        self.shell.set_cursor_icon(icon);
                    }
                }
            }
        }
    }

    fn start_key_event<W>(&mut self, widget: &mut W, vkey: VirtualKeyCode, scancode: u32)
    where
        W: Widget<Msg = VoidMsg> + ?Sized,
    {
        trace!(
            "EventMgr::start_key_event: widget={}, vkey={:?}, scancode={}",
            widget.id(),
            vkey,
            scancode
        );

        use VirtualKeyCode as VK;
        let shift = self.state.modifiers.shift();

        let opt_command = self
            .state
            .config
            .borrow()
            .shortcuts()
            .get(self.state.modifiers, vkey);

        if let Some(cmd) = opt_command {
            if self.state.char_focus {
                if let Some(id) = self.state.sel_focus.clone() {
                    if self.try_send_event(widget, id, Event::Command(cmd, shift)) {
                        return;
                    }
                }
            }

            if !self.state.modifiers.alt() {
                if let Some(id) = self.state.nav_focus.clone() {
                    if self.try_send_event(widget, id, Event::Command(cmd, shift)) {
                        return;
                    }
                }
            }

            if let Some(id) = self.state.popups.last().map(|popup| popup.1.parent.clone()) {
                if self.try_send_event(widget, id, Event::Command(cmd, shift)) {
                    return;
                }
            }

            if self.state.sel_focus != self.state.nav_focus && cmd.suitable_for_sel_focus() {
                if let Some(id) = self.state.sel_focus.clone() {
                    if self.try_send_event(widget, id, Event::Command(cmd, shift)) {
                        return;
                    }
                }
            }

            if let Some(id) = self.state.nav_fallback.clone() {
                if self.try_send_event(widget, id, Event::Command(cmd, shift)) {
                    return;
                }
            }
        }

        // Next priority goes to accelerator keys when Alt is held or alt_bypass is true
        let mut target = None;
        let mut n = 0;
        for (i, id) in (self.state.popups.iter().rev())
            .map(|(_, popup, _)| popup.parent.clone())
            .chain(std::iter::once(widget.id()))
            .enumerate()
        {
            if let Some(layer) = self.state.accel_layers.get(&id) {
                // but only when Alt is held or alt-bypass is enabled:
                if self.state.modifiers.alt() || layer.0 {
                    if let Some(id) = layer.1.get(&vkey).cloned() {
                        target = Some(id);
                        n = i;
                        break;
                    }
                }
            }
        }

        // If we found a key binding below the top layer, we should close everything above
        if n > 0 {
            let last = self.state.popups.len() - 1;
            for i in 0..n {
                let id = self.state.popups[last - i].0;
                self.close_window(id, false);
            }
        }

        if let Some(id) = target {
            if widget
                .find_widget(&id)
                .map(|w| w.key_nav())
                .unwrap_or(false)
            {
                self.set_nav_focus(id.clone(), true);
            }
            self.add_key_depress(scancode, id.clone());
            self.send_event(widget, id, Event::Activate);
        } else if vkey == VK::Tab {
            self.clear_char_focus();
            self.next_nav_focus(widget.as_widget_mut(), shift, true);
        } else if vkey == VK::Escape {
            if let Some(id) = self.state.popups.last().map(|(id, _, _)| *id) {
                self.close_window(id, true);
            }
        } else if !self.state.char_focus {
            if let Some(id) = self.state.nav_focus.clone() {
                if vkey == VK::Space || vkey == VK::Return || vkey == VK::NumpadEnter {
                    self.add_key_depress(scancode, id.clone());
                    self.send_event(widget, id, Event::Activate);
                }
            }
        }
    }

    fn add_key_depress(&mut self, scancode: u32, id: WidgetId) {
        if self.state.key_depress.values().any(|v| *v == id) {
            return;
        }

        self.state.key_depress.insert(scancode, id.clone());
        self.redraw(id);
    }

    fn end_key_event(&mut self, scancode: u32) {
        // We must match scancode not vkey since the latter may have changed due to modifiers
        if let Some(id) = self.state.key_depress.remove(&scancode) {
            self.redraw(id);
        }
    }

    fn mouse_grab(&mut self) -> Option<&mut MouseGrab> {
        self.state.mouse_grab.as_mut()
    }

    fn end_mouse_grab(&mut self, button: MouseButton) {
        if self
            .state
            .mouse_grab
            .as_ref()
            .map(|grab| grab.button != button)
            .unwrap_or(true)
        {
            return;
        }
        if let Some(grab) = self.state.mouse_grab.take() {
            trace!("EventMgr: end mouse grab by {}", grab.start_id);
            self.shell.set_cursor_icon(self.state.hover_icon);
            self.redraw(grab.start_id);
            self.state.remove_pan_grab(grab.pan_grab);
        }
    }

    #[inline]
    fn get_touch(&mut self, touch_id: u64) -> Option<&mut TouchGrab> {
        for grab in self.state.touch_grab.iter_mut() {
            if grab.id == touch_id {
                return Some(grab);
            }
        }
        None
    }

    fn remove_touch(&mut self, touch_id: u64) -> Option<TouchGrab> {
        for i in 0..self.state.touch_grab.len() {
            if self.state.touch_grab[i].id == touch_id {
                trace!(
                    "EventMgr: end touch grab by {}",
                    self.state.touch_grab[i].start_id
                );
                return Some(self.state.touch_grab.remove(i));
            }
        }
        None
    }

    fn clear_char_focus(&mut self) {
        trace!("EventMgr::clear_char_focus");
        if let Some(id) = self.state.char_focus() {
            // If widget has char focus, this is lost
            self.state.char_focus = false;
            self.state.pending.push(Pending::LostCharFocus(id));
        }
    }

    // Set selection focus to `wid`; if `char_focus` also set that
    fn set_sel_focus(&mut self, wid: WidgetId, char_focus: bool) {
        trace!(
            "EventMgr::set_sel_focus: wid={}, char_focus={}",
            wid,
            char_focus
        );
        // The widget probably already has nav focus, but anyway:
        self.set_nav_focus(wid.clone(), true);

        if wid == self.state.sel_focus {
            self.state.char_focus = self.state.char_focus || char_focus;
            return;
        }

        if let Some(id) = self.state.sel_focus.clone() {
            if self.state.char_focus {
                // If widget has char focus, this is lost
                self.state.pending.push(Pending::LostCharFocus(id.clone()));
            }

            // Selection focus is lost if another widget receives char focus
            self.state.pending.push(Pending::LostSelFocus(id));
        }

        self.state.char_focus = char_focus;
        self.state.sel_focus = Some(wid);
    }

    fn send_event<W: Widget + ?Sized>(&mut self, widget: &mut W, id: WidgetId, event: Event) {
        trace!("Send to {}: {:?}", id, event);
        let _ = widget.send(self, id, event);
    }

    // Similar to send_event, but return true only if response != Response::Unused
    fn try_send_event<W: Widget + ?Sized>(
        &mut self,
        widget: &mut W,
        id: WidgetId,
        event: Event,
    ) -> bool {
        trace!("Send to {}: {:?}", id, event);
        let r = widget.send(self, id, event);
        !matches!(r, Response::Unused)
    }

    fn send_popup_first<W: Widget + ?Sized>(&mut self, widget: &mut W, id: WidgetId, event: Event) {
        while let Some((wid, parent)) = self
            .state
            .popups
            .last()
            .map(|(wid, p, _)| (*wid, p.parent.clone()))
        {
            trace!("Send to popup parent: {}: {:?}", parent, event);
            match widget.send(self, parent, event.clone()) {
                Response::Unused => (),
                _ => return,
            }
            self.close_window(wid, false);
        }
        self.send_event(widget, id, event);
    }
}

/// Helper used during widget configuration
pub struct ConfigureManager<'a: 'b, 'b> {
    count: &'b mut usize,
    used: bool,
    id: WidgetId,
    mgr: &'b mut EventMgr<'a>,
}

impl<'a: 'b, 'b> ConfigureManager<'a, 'b> {
    /// Reborrow self to pass to a child
    ///
    /// The child's `index` becomes part of the identifier, and hence must be
    /// unique.
    pub fn child<'c>(&'c mut self, index: usize) -> ConfigureManager<'a, 'c>
    where
        'b: 'c,
    {
        ConfigureManager {
            count: &mut *self.count,
            used: false,
            id: self.id.make_child(index),
            mgr: &mut *self.mgr,
        }
    }

    /// Get [`WidgetId`] for self
    ///
    /// Do not call more than once on each instance. Create a new instance with
    /// [`Self::child`].
    pub fn get_id(&mut self) -> WidgetId {
        assert!(
            !self.used,
            "multiple use of ConfigureManager::get_id without construction of child"
        );
        self.used = true;
        self.id.clone()
    }

    /// Get access to the wrapped [`EventMgr`]
    pub fn mgr(&mut self) -> &mut EventMgr<'a> {
        self.mgr
    }
}
