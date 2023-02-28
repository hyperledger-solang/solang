// SPDX-License-Identifier: Apache-2.0

use crate::build_solidity;
use borsh::BorshDeserialize;
use sha2::{Digest, Sha256};

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
        }"#,
    );

    vm.constructor(&[]);

    vm.function("go", &[]);

    assert_eq!(vm.events.len(), 1);
    assert_eq!(vm.events[0].len(), 1);

    let encoded = &vm.events[0][0];

    let discriminator = calculate_discriminator("myevent");

    assert_eq!(&encoded[..8], &discriminator[..]);

    let decoded = MyEvent::try_from_slice(&encoded[8..]).unwrap();
    assert_eq!(decoded.a, 1);
    assert_eq!(decoded.b, -2);
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
        }"#,
    );

    vm.constructor(&[]);

    vm.function("go", &[]);

    assert_eq!(vm.events.len(), 1);
    assert_eq!(vm.events[0].len(), 1);

    let encoded = &vm.events[0][0];

    let discriminator = calculate_discriminator("MyOtherEvent");
    assert_eq!(&encoded[..8], &discriminator[..]);

    let decoded = MyOtherEvent::try_from_slice(&encoded[8..]).unwrap();

    assert_eq!(decoded.a, -102);
    assert_eq!(decoded.b, "foobar");
    assert_eq!(decoded.c, [55431, 7452]);
    assert_eq!(decoded.d, S { f1: 102, f2: true });
}

fn calculate_discriminator(event_name: &str) -> Vec<u8> {
    let image = format!("event:{event_name}");
    let mut hasher = Sha256::new();
    hasher.update(image.as_bytes());
    let finalized = hasher.finalize();
    finalized[..8].to_vec()
}
