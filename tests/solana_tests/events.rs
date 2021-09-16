use crate::build_solidity;
use ethabi::Token;
use tiny_keccak::{Hasher, Keccak};

#[test]
fn simple_event() {
    let mut vm = build_solidity(
        r#"
        contract c {
            event e(int indexed a, int b);

            function go() public {
                emit e(1, 2);
            }
        }"#,
    );

    vm.constructor("c", &[]);

    vm.function("go", &[], &[]);

    let log = vm.events();

    assert_eq!(log.len(), 1);

    let program = &vm.stack[0];

    let abi = program.abi.as_ref().unwrap();

    let event = &abi.events_by_name("e").unwrap()[0];

    assert_eq!(log[0].topics[0], event.signature());

    let decoded = event.parse_log(log[0].clone()).unwrap();

    for log in &decoded.params {
        match log.name.as_str() {
            "a" => assert_eq!(log.value, Token::Int(ethereum_types::U256::from(1))),
            "b" => assert_eq!(log.value, Token::Int(ethereum_types::U256::from(2))),
            _ => panic!("unexpected field {}", log.name),
        }
    }
}

#[test]
fn less_simple_event() {
    let mut vm = build_solidity(
        r#"
        contract c {
            struct S {
                int64 f1;
                bool f2;
            }

            event e(
                int indexed a,
                string indexed b,
                int[2] indexed c,
                S d);

            function go() public {
                emit e(-102, "foobar", [1, 2], S({ f1: 102, f2: true}));
            }
        }"#,
    );

    vm.constructor("c", &[]);

    vm.function("go", &[], &[]);

    let log = vm.events();

    assert_eq!(log.len(), 1);

    let program = &vm.stack[0];

    let abi = program.abi.as_ref().unwrap();

    let event = &abi.events_by_name("e").unwrap()[0];

    assert_eq!(log[0].topics[0], event.signature());

    let decoded = event.parse_log(log[0].clone()).unwrap();

    for log in &decoded.params {
        match log.name.as_str() {
            "a" => assert_eq!(
                log.value,
                Token::Int(ethereum_types::U256::from_dec_str("115792089237316195423570985008687907853269984665640564039457584007913129639834").unwrap())
            ),
            "b" => {
                let mut hasher = Keccak::v256();
                hasher.update(b"foobar");
                let mut hash = [0u8; 32];
                hasher.finalize(&mut hash);

                assert_eq!(log.value, Token::FixedBytes(hash.to_vec()));
            }
            "c" => {
                let mut hasher = Keccak::v256();
                let mut v = [0u8; 32];
                v[31] = 1;
                hasher.update(&v);
                v[31] = 2;
                hasher.update(&v);
                let mut hash = [0u8; 32];
                hasher.finalize(&mut hash);

                assert_eq!(log.value, Token::FixedBytes(hash.to_vec()));
            }
            "d" => {
                assert_eq!(log.value, Token::Tuple(vec![Token::Int(ethereum_types::U256::from(102)), Token::Bool(true)]));
            }

            _ => panic!("unexpected field {}", log.name),
        }
    }
}
