// SPDX-License-Identifier: Apache-2.0

#![cfg(test)]
use crate::sema::diagnostics::Diagnostics;
use crate::sema::expression::strings::unescape;

#[test]
fn test_unescape() {
    let s = r"\u00f3";
    let mut vec = Diagnostics::default();
    let (valid, res) = unescape(s, 0, 0, &mut vec);
    assert!(valid && vec.is_empty());
    assert_eq!(res, vec![0xc3, 0xb3]);

    let s = r"\xff";
    let (valid, res) = unescape(s, 0, 0, &mut vec);
    assert!(valid && vec.is_empty());
    assert_eq!(res, vec![255]);

    let s = r"0\xfg";
    let (valid, res) = unescape(s, 0, 0, &mut vec);
    assert!(!valid && !vec.is_empty());
    assert_eq!(res, b"0");
}
