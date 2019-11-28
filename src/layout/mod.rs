// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Layout solver
//!
//! This is only of interest if building a custom widget with children.

mod grid_solver;
mod row_solver;
mod size_rules;
mod sizer;

pub use grid_solver::{FixedGridSolver, GridChildInfo};
pub use row_solver::FixedRowSolver;
pub use size_rules::{Margins, SizeRules};
pub use sizer::{solve, Sizer};

/// Parameter type passed to [`Layout::size_rules`]
#[derive(Copy, Clone, Debug)]
pub struct AxisInfo {
    vertical: bool,
    has_fixed: bool,
    other_axis: u32,
}

impl AxisInfo {
    fn new(vertical: bool, fixed: Option<u32>) -> Self {
        AxisInfo {
            vertical: vertical,
            has_fixed: fixed.is_some(),
            other_axis: fixed.unwrap_or(0),
        }
    }

    /// True if the current axis is vertical, false if horizontal
    #[inline]
    pub fn vertical(&self) -> bool {
        self.vertical
    }

    /// Size of other axis, if fixed and `vertical == self.vertical()`.
    #[inline]
    pub fn fixed(&self, vertical: bool) -> Option<u32> {
        if vertical == self.vertical && self.has_fixed {
            Some(self.other_axis)
        } else {
            None
        }
    }
}
