// SPDX-License-Identifier: Apache-2.0

//! Implements `PartialOrd` and `Ord` for some parse tree data types, following the
//! [Solidity style guide][ref].
//!
//! [ref]: https://docs.soliditylang.org/en/latest/style-guide.html

use crate::pt;
use std::cmp::Ordering;

macro_rules! impl_with_cast {
    ($($t:ty),+) => {
        $(
            impl $t {
                #[inline]
                const fn as_discriminant(&self) -> &u8 {
                    // SAFETY: See <https://doc.rust-lang.org/stable/std/mem/fn.discriminant.html#accessing-the-numeric-value-of-the-discriminant>
                    // and <https://doc.rust-lang.org/reference/items/enumerations.html#pointer-casting>
                    //
                    // `$t` must be `repr(u8)` for this to be safe.
                    unsafe { &*(self as *const Self as *const u8) }
                }
            }

            impl PartialOrd for $t {
                #[inline]
                fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
                    PartialOrd::partial_cmp(self.as_discriminant(), other.as_discriminant())
                }
            }

            impl Ord for $t {
                #[inline]
                fn cmp(&self, other: &Self) -> Ordering {
                    Ord::cmp(self.as_discriminant(), other.as_discriminant())
                }
            }
        )+
    };
}

// SAFETY: every type must be `repr(u8)` for this to be safe, see comments in macro implementation.
impl_with_cast!(pt::Visibility, pt::VariableAttribute, pt::FunctionAttribute);
