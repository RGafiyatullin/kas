// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Theme configuration

use crate::ThemeColours;
use kas::TkAction;
#[cfg(feature = "serde")]
use std::collections::BTreeMap;

/// Event handling configuration
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Config {
    /// Standard font size
    ///
    /// Units: points per Em. Pixel size depends on the screen's scale factor.
    #[cfg_attr(feature = "serde", serde(default = "defaults::font_size"))]
    pub font_size: f32,

    /// Active colour scheme (name)
    ///
    /// An empty string will resolve the default colour scheme.
    #[cfg_attr(feature = "serde", serde(default))]
    pub color_scheme: String,

    /// All colour schemes
    ///
    /// TODO: possibly we should not save default schemes and merge when
    /// loading (perhaps via a `PartialConfig` type).
    #[cfg_attr(feature = "serde", serde(default = "defaults::color_schemes",))]
    pub color_schemes: BTreeMap<String, ThemeColours>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            font_size: defaults::font_size(),
            color_scheme: Default::default(),
            color_schemes: defaults::color_schemes(),
        }
    }
}

impl Config {
    /// Currently this is just "set". Later, maybe some type of merge.
    pub fn apply_config(&mut self, other: &Config) -> TkAction {
        let action = if self.font_size != other.font_size {
            TkAction::RESIZE | TkAction::THEME_UPDATE
        } else if self != other {
            TkAction::REDRAW
        } else {
            TkAction::empty()
        };

        *self = other.clone();
        action
    }
}

mod defaults {
    use super::*;

    pub fn font_size() -> f32 {
        12.0
    }

    pub fn color_schemes() -> BTreeMap<String, ThemeColours> {
        let mut schemes = BTreeMap::new();
        schemes.insert("".to_string(), ThemeColours::white_blue());
        schemes.insert("light".to_string(), ThemeColours::light());
        schemes.insert("dark".to_string(), ThemeColours::dark());
        schemes
    }
}
