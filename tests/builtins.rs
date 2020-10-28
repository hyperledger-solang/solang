extern crate solang;

use solang::sema::ast;
use solang::sema::builtin;

#[test]
fn builtin_prototype() {
    let p = builtin::get_prototype(ast::Builtin::Timestamp).unwrap();

    assert_eq!(p.namespace, Some("block"));
    assert_eq!(p.name, "timestamp");
    assert!(p.args.is_empty());

    // there is no entry for some builtins
    assert!(builtin::get_prototype(ast::Builtin::ArrayPush).is_none());
}
