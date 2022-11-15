// SPDX-License-Identifier: Apache-2.0

use crate::build_solidity;
use ink_primitives::Hash;
use parity_scale_codec::{self as scale, Decode, Encode};
use solang::{file_resolver::FileResolver, Target};
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

    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
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

/// Taken from: Erc20 example test in ink!
struct PrefixedValue<'a, 'b, T> {
    pub prefix: &'a [u8],
    pub value: &'b T,
}

impl<X> scale::Encode for PrefixedValue<'_, '_, X>
where
    X: scale::Encode,
{
    #[inline]
    fn size_hint(&self) -> usize {
        self.prefix.size_hint() + self.value.size_hint()
    }

    #[inline]
    fn encode_to<T: scale::Output + ?Sized>(&self, dest: &mut T) {
        self.prefix.encode_to(dest);
        self.value.encode_to(dest);
    }
}

fn encoded_into_hash<T>(entity: &T) -> Hash
where
    T: scale::Encode,
{
    use ink_env::hash::{Blake2x256, CryptoHash, HashOutput};
    use ink_primitives::Clear;

    let mut result = Hash::clear();
    let len_result = result.as_ref().len();
    let encoded = entity.encode();
    let len_encoded = encoded.len();
    if len_encoded <= len_result {
        result.as_mut()[..len_encoded].copy_from_slice(&encoded);
        return result;
    }
    let mut hash_output = <<Blake2x256 as HashOutput>::Type as Default>::default();
    <Blake2x256 as CryptoHash>::hash(&encoded, &mut hash_output);
    let copy_len = core::cmp::min(hash_output.len(), len_result);
    result.as_mut()[0..copy_len].copy_from_slice(&hash_output[0..copy_len]);
    result
}
