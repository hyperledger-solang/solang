// SPDX-License-Identifier: Apache-2.0

use crate::{borsh_encoding::BorshToken, build_solidity};
use borsh::BorshDeserialize;
use borsh_derive::BorshDeserialize;
use solang::abi::anchor::event_discriminator;

#[test]
fn simple_event() {
    #[derive(BorshDeserialize, PartialEq, Eq, Debug)]
    struct MyEvent {
        a: i32,
        b: i32,
    }

    let mut vm = build_solidity(
        r#"
        contract c {
            event myevent(int32 indexed a, int32 b);

            function go() public {
                emit myevent(1, -2);
            }

            function selector() public returns (bytes8) {
                return myevent.selector;
            }
        }"#,
    );

    let data_account = vm.initialize_data_account();
    vm.function("new")
        .accounts(vec![("dataAccount", data_account)])
        .call();

    vm.function("go").call();

    assert_eq!(vm.events.len(), 1);
    assert_eq!(vm.events[0].len(), 1);

    let encoded = &vm.events[0][0];

    let discriminator = event_discriminator("myevent");

    assert_eq!(&encoded[..8], &discriminator[..]);

    let decoded = MyEvent::try_from_slice(&encoded[8..]).unwrap();
    assert_eq!(decoded.a, 1);
    assert_eq!(decoded.b, -2);

    let returns = vm.function("selector").call().unwrap();

    assert_eq!(
        returns,
        BorshToken::FixedArray(
            discriminator
                .into_iter()
                .map(|v| BorshToken::Uint {
                    width: 8,
                    value: v.into()
                })
                .collect()
        )
    );
}

#[test]
fn less_simple_event() {
    #[derive(BorshDeserialize, PartialEq, Eq, Debug)]
    struct S {
        f1: i64,
        f2: bool,
    }

    #[derive(BorshDeserialize, PartialEq, Eq, Debug)]
    struct MyOtherEvent {
        a: i16,
        b: String,
        c: [i128; 2],
        d: S,
    }

    let mut vm = build_solidity(
        r#"
        contract c {
            struct S {
                int64 f1;
                bool f2;
            }

            event MyOtherEvent(
                int16 indexed a,
                string indexed b,
                uint128[2] indexed c,
                S d);

            function go() public {
                emit MyOtherEvent(-102, "foobar", [55431, 7452], S({ f1: 102, f2: true}));
            }

            function selector() public returns (bytes8) {
                return MyOtherEvent.selector;
            }
        }"#,
    );

    let data_account = vm.initialize_data_account();
    vm.function("new")
        .accounts(vec![("dataAccount", data_account)])
        .call();

    vm.function("go").call();

    assert_eq!(vm.events.len(), 1);
    assert_eq!(vm.events[0].len(), 1);

    let encoded = &vm.events[0][0];

    let discriminator = event_discriminator("MyOtherEvent");
    assert_eq!(&encoded[..8], &discriminator[..]);

    let decoded = MyOtherEvent::try_from_slice(&encoded[8..]).unwrap();

    assert_eq!(decoded.a, -102);
    assert_eq!(decoded.b, "foobar");
    assert_eq!(decoded.c, [55431, 7452]);
    assert_eq!(decoded.d, S { f1: 102, f2: true });

    let returns = vm.function("selector").call().unwrap();

    assert_eq!(
        returns,
        BorshToken::FixedArray(
            discriminator
                .into_iter()
                .map(|v| BorshToken::Uint {
                    width: 8,
                    value: v.into()
                })
                .collect()
        )
    );
}
