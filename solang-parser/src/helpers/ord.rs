// SPDX-License-Identifier: Apache-2.0

//! Implements `PartialOrd` and `Ord` for some parse tree data types, following the
//! [Solidity style guide][ref].
//!
//! [ref]: https://docs.soliditylang.org/en/latest/style-guide.html

use crate::pt;
use std::cmp::Ordering;

macro_rules! impl_with_sort_key {
    ($($t:ty),+) => {
        $(
            impl PartialOrd for $t {
                #[inline]
                fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
                    Some(Ord::cmp(self, other))
                }
            }

            impl Ord for $t {
                fn cmp(&self, other: &Self) -> Ordering {
                    Ord::cmp(&self.sort_key(), &other.sort_key())
                }
            }
        )+
    };
}

impl_with_sort_key!(pt::Visibility, pt::VariableAttribute, pt::FunctionAttribute);

impl pt::Visibility {
    #[inline]
    fn sort_key(&self) -> u8 {
        match self {
            pt::Visibility::External(..) => 0,
            pt::Visibility::Public(..) => 1,
            pt::Visibility::Internal(..) => 2,
            pt::Visibility::Private(..) => 3,
        }
    }
}

impl pt::VariableAttribute {
    #[inline]
    fn sort_key(&self) -> u8 {
        match self {
            pt::VariableAttribute::Visibility(..) => 0,
            pt::VariableAttribute::Constant(..) => 1,
            pt::VariableAttribute::Immutable(..) => 2,
            pt::VariableAttribute::Override(..) => 3,
        }
    }
}

impl pt::FunctionAttribute {
    #[inline]
    fn sort_key(&self) -> u8 {
        match self {
            pt::FunctionAttribute::Visibility(..) => 0,
            pt::FunctionAttribute::Mutability(..) => 1,
            pt::FunctionAttribute::Virtual(..) => 2,
            pt::FunctionAttribute::Immutable(..) => 3,
            pt::FunctionAttribute::Override(..) => 4,
            pt::FunctionAttribute::BaseOrModifier(..) => 5,
            pt::FunctionAttribute::Error(..) => 6, // supposed to be omitted even if sorted
        }
    }
}
