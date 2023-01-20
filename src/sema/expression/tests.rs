// SPDX-License-Identifier: Apache-2.0

#![cfg(test)]
use crate::sema::diagnostics::Diagnostics;
use crate::sema::expression::strings::unescape;

#[test]
fn test_unescape() {
    let s = r#"\u00f3"#;
    let mut vec = Diagnostics::default();
    let res = unescape(s, 0, 0, &mut vec);
    assert!(vec.is_empty());
    assert_eq!(res, vec![0xc3, 0xb3]);
    let s = r#"\xff"#;
    let res = unescape(s, 0, 0, &mut vec);
    assert!(vec.is_empty());
    assert_eq!(res, vec![255]);
}
