use crate::{build_solidity, no_errors};
use parity_scale_codec::Encode;
use parity_scale_codec_derive::Decode;
use solang::{file_resolver::FileResolver, Options, Target};
use std::ffi::OsStr;

#[test]
fn emit() {
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

    let ns = solang::parse_and_resolve(
        OsStr::new("a.sol"),
        &mut cache,
        Target::default_substrate(),
        &Options::default(),
    );

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

    let ns = solang::parse_and_resolve(
        OsStr::new("a.sol"),
        &mut cache,
        Target::default_substrate(),
        &Options::default(),
    );

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

    let ns = solang::parse_and_resolve(
        OsStr::new("a.sol"),
        &mut cache,
        Target::default_substrate(),
        &Options::default(),
    );

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

    let ns = solang::parse_and_resolve(
        OsStr::new("a.sol"),
        &mut cache,
        Target::default_substrate(),
        &Options::default(),
    );

    no_errors(ns.diagnostics);
}
