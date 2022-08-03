// SPDX-License-Identifier: Apache-2.0

use solang::sema::ast;
use solang::sema::builtin;

#[test]
fn builtin_prototype() {
    let p = builtin::get_prototype(ast::Builtin::Timestamp).unwrap();

    assert_eq!(p.namespace, Some("block"));
    assert_eq!(p.name, "timestamp");
    assert!(p.params.is_empty());
}
