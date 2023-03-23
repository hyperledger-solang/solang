// SPDX-License-Identifier: Apache-2.0

use solang_parser::pt::Loc;
use solang_parser::pt::OptionalCodeLocation;

/// This struct manages expressions assigned to an existing variable.
#[derive(Clone)]
pub enum AvailableVariable {
    // Variable for an expression is available
    Available(usize, Loc),
    // Variable for an expression has been invalidate during a branch.
    /*
    e.g.
    if(condition) {
        x = a+b;
    } else {
        x = a-b;
    }
    // x is invalidated
     */
    Invalidated,
    // When there is no variable available for the current expressions
    Unavailable,
}

impl AvailableVariable {
    pub fn get_var_number(&self) -> Option<usize> {
        match self {
            AvailableVariable::Available(number, _) => Some(*number),
            _ => None,
        }
    }

    pub fn is_available(&self) -> bool {
        matches!(self, AvailableVariable::Available(..))
    }

    pub fn is_invalid(&self) -> bool {
        matches!(self, AvailableVariable::Invalidated)
    }
}

impl OptionalCodeLocation for AvailableVariable {
    fn loc_opt(&self) -> Option<Loc> {
        match self {
            AvailableVariable::Available(_, loc) => Some(*loc),
            _ => None,
        }
    }
}
