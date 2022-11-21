// SPDX-License-Identifier: Apache-2.0

use crate::build_solidity;
use ink_env::{
    hash::{Blake2x256, CryptoHash},
    topics::PrefixedValue,
};
use parity_scale_codec::{self as scale, Decode, Encode};
use solang::{file_resolver::FileResolver, Target};
use std::ffi::OsStr;

pub(crate) fn topic_hash(encoded: &[u8]) -> Vec<u8> {
    let mut buf = [0; 32];
    if encoded.len() <= 32 {
        buf[..encoded.len()].copy_from_slice(encoded);
    } else {
        <Blake2x256 as CryptoHash>::hash(encoded, &mut buf);
    };
    buf.into()
}

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

    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    enum Event {
        Foo(bool, u32, i64),
    }

    let mut runtime = build_solidity(
        r##"
        contract a {
            event foo(bool,uint32,int64 indexed i);
            event bar(uint32,uint64,string indexed s);
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
    assert_eq!(event.topics.len(), 2);
    let mut t = [0u8; 32];
    t[0] = 1;
    let mut event_topic = scale::Encode::encode(&String::from("a::foo"));
    event_topic[0] = 0;
    assert_eq!(event.topics[0], topic_hash(&event_topic[..])[..]);
    let topic = PrefixedValue {
        prefix: b"a::foo::i",
        value: &1i64,
    }
    .encode();
    assert_eq!(event.topics[1], topic_hash(&topic[..])[..]);
    assert_eq!(event.data, Event::Foo(true, 102, 1).encode());

    let event = &runtime.events[1];
    assert_eq!(event.topics.len(), 2);
    println!(
        "topic hash: {}",
        std::str::from_utf8(&event.topics[0]).unwrap()
    );
    println!(
        "topic hash: {}",
        std::str::from_utf8(&event.topics[0]).unwrap()
    );
    let mut event_topic = scale::Encode::encode(&String::from("a::bar"));
    event_topic[0] = 0;
    assert_eq!(event.topics[0], topic_hash(&event_topic[..])[..]);
    let topic = PrefixedValue {
        prefix: b"a::bar::s",
        value: &String::from("foobar"),
    }
    .encode();
    assert_eq!(event.topics[1].to_vec(), topic_hash(&topic[..]));
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

    let ns =
        solang::parse_and_resolve(OsStr::new("a.sol"), &mut cache, Target::default_substrate());

    assert!(!ns.diagnostics.any_errors());

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
        abstract contract baz {
            event bar (uint32 indexed f1, bool x);
        }
        "#
        .to_string(),
    );

    let ns =
        solang::parse_and_resolve(OsStr::new("a.sol"), &mut cache, Target::default_substrate());

    assert!(!ns.diagnostics.any_errors());

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
        abstract contract baz {
            event bar (uint32 indexed f1, bool x);
        }
        "#
        .to_string(),
    );

    let ns =
        solang::parse_and_resolve(OsStr::new("a.sol"), &mut cache, Target::default_substrate());

    assert!(!ns.diagnostics.any_errors());

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

    let ns =
        solang::parse_and_resolve(OsStr::new("a.sol"), &mut cache, Target::default_substrate());

    assert!(!ns.diagnostics.any_errors());
}
