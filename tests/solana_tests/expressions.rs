use crate::build_solidity;
use ethabi::Token;
use rand::Rng;

#[test]
fn interfaceid() {
    let mut vm = build_solidity(
        r#"
        contract foo {
            function get() public returns (bytes4) {
                return type(I).interfaceId;
            }
        }

        interface I {
            function bar(int) external;
            function baz(bytes) external returns (int);
        }"#,
    );

    vm.constructor("foo", &[], 0);

    let returns = vm.function("get", &[], &[], 0, None);

    assert_eq!(
        returns,
        vec![Token::FixedBytes(0xc78d9f3au32.to_be_bytes().to_vec())]
    );
}

#[test]
fn write_buffer() {
    let mut vm = build_solidity(
        r#"
        contract foo {
            function test1() public returns (bytes) {
                bytes bs = new bytes(12);
                bs.writeInt32LE(-0x41424344, 0);
                bs.writeUint64LE(0x0102030405060708, 4);
                return bs;
            }

            function test2() public returns (bytes) {
                bytes bs = new bytes(34);
                bs.writeUint16LE(0x4142, 0);
                bs.writeAddress(msg.sender, 2);
                return bs;
            }

            function test3() public returns (bytes) {
                bytes bs = new bytes(9);
                bs.writeUint64LE(1, 2);
                return bs;
            }
        }"#,
    );

    vm.constructor("foo", &[], 0);

    let returns = vm.function("test1", &[], &[], 0, None);

    assert_eq!(
        returns,
        vec![Token::Bytes(
            [0xbc, 0xbc, 0xbd, 0xbe, 8, 7, 6, 5, 4, 3, 2, 1].to_vec()
        )]
    );

    let returns = vm.function("test2", &[], &[], 0, None);

    let mut buf = vec![0x42u8, 0x41u8];
    buf.extend_from_slice(&vm.origin);

    assert_eq!(returns, vec![Token::Bytes(buf)]);

    let res = vm.function_must_fail("test3", &[], &[], 0, None);
    assert_eq!(res, Ok(4294967296));
}

#[test]
fn read_buffer() {
    let mut vm = build_solidity(
        r#"
        contract foo {
            function test1(bytes bs) public returns (int32, uint64) {
                return (bs.readInt32LE(0), bs.readUint64LE(4));
            }

            function test2(bytes bs) public returns (uint16, address) {
                return (bs.readUint16LE(0), bs.readAddress(2));
            }
        }"#,
    );

    vm.constructor("foo", &[], 0);

    let returns = vm.function(
        "test1",
        &[Token::Bytes(
            [0xbc, 0xbc, 0xbd, 0xbe, 8, 7, 6, 5, 4, 3, 2, 1].to_vec(),
        )],
        &[],
        0,
        None,
    );

    assert_eq!(
        returns,
        vec![
            Token::Int(ethereum_types::U256::from(
                "0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffbebdbcbc"
            )),
            Token::Uint(ethereum_types::U256::from(0x0102030405060708u64))
        ]
    );

    let res = vm.function_must_fail(
        "test1",
        &[Token::Bytes(
            [0xbc, 0xbc, 0xbd, 0xbe, 8, 7, 6, 5, 4, 3, 2].to_vec(),
        )],
        &[],
        0,
        None,
    );
    assert_eq!(res, Ok(4294967296));

    let mut buf = vec![0x42u8, 0x41u8];
    buf.extend_from_slice(&vm.origin);

    let returns = vm.function("test2", &[Token::Bytes(buf.clone())], &[], 0, None);

    assert_eq!(
        returns,
        vec![
            Token::Uint(ethereum_types::U256::from(0x4142)),
            Token::FixedBytes(vm.origin.to_vec())
        ]
    );

    buf.pop();

    let res = vm.function_must_fail("test2", &[Token::Bytes(buf)], &[], 0, None);
    assert_eq!(res, Ok(4294967296));
}

#[test]
fn bytes_compare() {
    let mut vm = build_solidity(
        r#"
        contract foo {
            function test1(bytes4 bs) public returns (bool) {
                return bs != 0;
            }

            function test2(bytes4 bs) public returns (bool) {
                return bs == 0;
            }
        }"#,
    );

    vm.constructor("foo", &[], 0);

    let returns = vm.function(
        "test1",
        &[Token::FixedBytes([0xbc, 0xbc, 0xbd, 0xbe].to_vec())],
        &[],
        0,
        None,
    );

    assert_eq!(returns, vec![Token::Bool(true)]);

    let returns = vm.function(
        "test2",
        &[Token::FixedBytes([0xbc, 0xbc, 0xbd, 0xbe].to_vec())],
        &[],
        0,
        None,
    );

    assert_eq!(returns, vec![Token::Bool(false)]);
}

#[test]
fn assignment_in_ternary() {
    let mut rng = rand::thread_rng();

    let mut vm = build_solidity(
        r#"
        contract foo {
            function minimum(uint64 x, uint64 y) public pure returns (uint64 z) {
                x >= y ? z = y : z = x;
            }
        }"#,
    );

    vm.constructor("foo", &[], 0);

    for _ in 0..10 {
        let left = rng.gen::<u64>();
        let right = rng.gen::<u64>();

        let returns = vm.function(
            "minimum",
            &[
                Token::Uint(ethereum_types::U256::from(left)),
                Token::Uint(ethereum_types::U256::from(right)),
            ],
            &[],
            0,
            None,
        );

        assert_eq!(
            returns,
            vec![Token::Uint(ethereum_types::U256::from(std::cmp::min(
                left, right
            )))]
        );
    }
}
