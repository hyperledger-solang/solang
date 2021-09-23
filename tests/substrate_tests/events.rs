use crate::{build_solidity, first_error, first_warning, no_errors, parse_and_resolve};
use parity_scale_codec::Encode;
use parity_scale_codec_derive::{Decode, Encode};
use solang::file_resolver::FileResolver;
use solang::Target;

#[test]
fn event_decl() {
    let ns = parse_and_resolve(
        r#"
        contract c {
            event foo ();
        }"#,
        Target::Substrate,
    );

    no_errors(ns.diagnostics);

    let ns = parse_and_resolve(
        r#"
        enum e { a1 }
        event e();"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "e is already defined as an enum"
    );

    let ns = parse_and_resolve(
        r#"
        enum e { a1 }
        contract c {
            event e();
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_warning(ns.diagnostics),
        "e is already defined as an enum"
    );

    let ns = parse_and_resolve(
        r#"
        contract c {
            enum e { a1 }
            event e();
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "e is already defined as an enum"
    );

    let ns = parse_and_resolve(
        r#"
        contract c {
            event foo (mapping (bool => uint) x);
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "mapping type is not permitted as event field"
    );

    let ns = parse_and_resolve(
        r#"
        struct s {
            mapping (bool => uint) f1;
        }

        contract c {
            event foo (s x);
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "mapping type is not permitted as event field"
    );

    let ns = parse_and_resolve(
        r#"
        contract c {
            event foo (bool x, uint32 y, address x);
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "event ‘foo’ has duplicate field name ‘x’"
    );

    let ns = parse_and_resolve(
        r#"
        contract c {
            event foo (bool indexed f1, bool indexed f2, bool indexed f3, bool indexed f4);
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "event definition for ‘foo’ has 4 indexed fields where 3 permitted"
    );

    let ns = parse_and_resolve(
        r#"
        contract c {
            event foo (bool indexed f1, bool indexed f2, bool indexed f3);
        }"#,
        Target::Substrate,
    );

    no_errors(ns.diagnostics);

    let ns = parse_and_resolve(
        r#"
        contract c {
            event foo (bool indexed f1, bool indexed f2, bool indexed f3, bool indexed f4, bool indexed f5) anonymous;
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "anonymous event definition for ‘foo’ has 5 indexed fields where 4 permitted"
    );

    let ns = parse_and_resolve(
        r#"
        contract c {
            event foo (bool indexed f1, bool indexed f2, bool indexed f3, bool indexed f4) anonymous;
        }"#,
        Target::Substrate,
    );

    no_errors(ns.diagnostics);
}

#[test]
fn emit() {
    let ns = parse_and_resolve(
        r#"
        contract c {
            function f() public {
                emit 1 ();
            }
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "expression found where type expected"
    );

    let ns = parse_and_resolve(
        r#"
        contract c {
            event foo(bool);
            function f() public {
                emit foo {};
            }
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "expected event arguments, found code block"
    );

    let ns = parse_and_resolve(
        r#"
        contract c {
            event foo(bool,uint32);
            function f() public {
                emit foo (true);
            }
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "event type ‘foo’ has 2 fields, 1 provided"
    );

    let ns = parse_and_resolve(
        r#"
        contract c {
            event foo(bool,uint32);
            function f() public {
                emit foo (true, "ab");
            }
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "implicit conversion to uint32 from bytes2 not allowed"
    );

    let ns = parse_and_resolve(
        r#"
        contract c {
            event foo(bool,uint32);
            function f() public {
                emit foo ({a:true, a:"ab"});
            }
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "duplicate argument with name ‘a’"
    );

    let ns = parse_and_resolve(
        r#"
        contract c {
            event foo(bool,uint32);
            function f() public {
                emit foo ({a:true, b:"ab"});
            }
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "event ‘foo’ cannot emitted by argument name since argument 0 has no name"
    );

    let ns = parse_and_resolve(
        r#"
        contract c {
            event foo(bool,uint32);
            function f() view public {
                emit foo (true, 102);
            }
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "function declared ‘view’ but this expression writes to state"
    );

    let mut runtime = build_solidity(
        r##"
        contract a {
            event foo(bool) anonymous;
            function emit_event() public {
                emit foo(true);
            }
        }"##,
    );

    runtime.constructor(0, Vec::new());
    runtime.function("emit_event", Vec::new());

    assert_eq!(runtime.events.len(), 1);
    let event = &runtime.events[0];
    assert_eq!(event.topics.len(), 0);
    assert_eq!(event.data, (0u8, true).encode());

    #[derive(Debug, PartialEq, Encode, Decode)]
    struct Foo(u8, bool, u32);

    let mut runtime = build_solidity(
        r##"
        contract a {
            event foo(bool,uint32,int64 indexed);
            event bar(uint32,uint64,string indexed);
            function emit_event() public {
                emit foo(true, 102, 1);
                emit bar(0xdeadcafe, 102, "foobar");
            }
        }"##,
    );

    runtime.constructor(0, Vec::new());
    runtime.function("emit_event", Vec::new());

    assert_eq!(runtime.events.len(), 2);
    let event = &runtime.events[0];
    assert_eq!(event.topics.len(), 1);
    let mut t = [0u8; 32];
    t[0] = 1;

    assert_eq!(event.topics[0], t);
    assert_eq!(event.data, Foo(0, true, 102).encode());

    let event = &runtime.events[1];
    assert_eq!(event.topics.len(), 1);
    assert_eq!(
        event.topics[0].to_vec(),
        hex::decode("38d18acb67d25c8bb9942764b62f18e17054f66a817bd4295423adf9ed98873e").unwrap()
    );
    assert_eq!(event.data, (1u8, 0xdeadcafeu32, 102u64).encode());
}

#[test]
fn event_imported() {
    let mut cache = FileResolver::new();

    cache.set_file_contents(
        "a.sol",
        r#"
        import "b.sol";

        contract foo {
            function emit_event() public {
                emit bar(102, true);
            }
        }
        "#
        .to_string(),
    );

    cache.set_file_contents(
        "b.sol",
        r#"
        event bar (uint32 indexed f1, bool x);
        "#
        .to_string(),
    );

    let ns = solang::parse_and_resolve("a.sol", &mut cache, Target::Substrate);

    no_errors(ns.diagnostics);

    let mut cache = FileResolver::new();

    cache.set_file_contents(
        "a.sol",
        r#"
        import "b.sol";

        contract foo {
            function emit_event() public {
                emit baz.bar(102, true);
            }
        }
        "#
        .to_string(),
    );

    cache.set_file_contents(
        "b.sol",
        r#"
        contract baz {
            event bar (uint32 indexed f1, bool x);
        }
        "#
        .to_string(),
    );

    let ns = solang::parse_and_resolve("a.sol", &mut cache, Target::Substrate);

    no_errors(ns.diagnostics);

    let mut cache = FileResolver::new();

    cache.set_file_contents(
        "a.sol",
        r#"
        import "b.sol" as X;

        contract foo {
            function emit_event() public {
                emit X.baz.bar(102, true);
            }
        }
        "#
        .to_string(),
    );

    cache.set_file_contents(
        "b.sol",
        r#"
        contract baz {
            event bar (uint32 indexed f1, bool x);
        }
        "#
        .to_string(),
    );

    let ns = solang::parse_and_resolve("a.sol", &mut cache, Target::Substrate);

    no_errors(ns.diagnostics);

    let mut cache = FileResolver::new();

    cache.set_file_contents(
        "a.sol",
        r#"
        import "b.sol" as X;

        contract foo {
            function emit_event() public {
                emit X.bar(102, true);
            }
        }
        "#
        .to_string(),
    );

    cache.set_file_contents(
        "b.sol",
        r#"
        event bar (uint32 indexed f1, bool x);
        "#
        .to_string(),
    );

    let ns = solang::parse_and_resolve("a.sol", &mut cache, Target::Substrate);

    no_errors(ns.diagnostics);
}

#[test]
fn inherited() {
    let ns = parse_and_resolve(
        r#"
        contract base {
            event foo(bool a, int b);
        }

        contract c is base {
            function f() public {
                emit foo(true, 1);
            }
        }"#,
        Target::Substrate,
    );

    no_errors(ns.diagnostics);
}

#[test]
fn signatures() {
    let ns = parse_and_resolve(
        r#"
        event foo(bool a, int b);
        event bar(bool a, int b);

        contract c {
            event foo(int b);
            event bar(int b);

            function f() public {
                emit foo(true, 1);
            }
        }"#,
        Target::Substrate,
    );

    no_errors(ns.diagnostics);

    let ns = parse_and_resolve(
        r#"
        event foo(bool a, int b);
        event foo(bool x, int y);

        contract c {
            event foo(int b);

            function f() public {
                emit foo(true, 1);
            }
        }"#,
        Target::Substrate,
    );

    no_errors(ns.diagnostics);

    let ns = parse_and_resolve(
        r#"
        event foo(bool a, int b);

        contract c {
            event foo(int b);
            event foo(int x);

            function f() public {
                emit foo(true, 1);
            }
        }"#,
        Target::Substrate,
    );

    no_errors(ns.diagnostics);

    let ns = parse_and_resolve(
        r#"
        event foo(bool a, int b);

        contract c {
            event foo(bool x, int y);

            function f() public {
                emit foo(true, 1);
            }
        }"#,
        Target::Substrate,
    );

    no_errors(ns.diagnostics);
}
