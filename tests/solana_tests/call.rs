use crate::{build_solidity, Instruction, Pubkey};
use base58::FromBase58;
use ethabi::{ethereum_types::U256, Token};

#[test]
fn simple_external_call() {
    let mut vm = build_solidity(
        r#"
        contract bar0 {
            function test_bar(string v) public {
                print("bar0 says: " + v);
            }

            function test_other(bar1 x) public {
                x.test_bar("cross contract call");
            }
        }

        contract bar1 {
            function test_bar(string v) public {
                print("bar1 says: " + v);
            }
        }"#,
    );

    vm.constructor("bar1", &[], 0);

    vm.function(
        "test_bar",
        &[Token::String(String::from("yo"))],
        &[],
        0,
        None,
    );

    assert_eq!(vm.logs, "bar1 says: yo");

    vm.logs.truncate(0);

    let bar1_account = vm.stack[0].data;

    vm.set_program(0);

    vm.constructor("bar0", &[], 0);

    vm.function(
        "test_bar",
        &[Token::String(String::from("uncle beau"))],
        &[],
        0,
        None,
    );

    assert_eq!(vm.logs, "bar0 says: uncle beau");

    vm.logs.truncate(0);

    vm.function(
        "test_other",
        &[Token::FixedBytes(bar1_account.to_vec())],
        &[],
        0,
        None,
    );

    assert_eq!(vm.logs, "bar1 says: cross contract call");
}

#[test]
fn external_call_with_returns() {
    let mut vm = build_solidity(
        r#"
        contract bar0 {
            function test_other(bar1 x) public returns (int64) {
                return x.test_bar(7) + 5;
            }
        }

        contract bar1 {
            function test_bar(int64 y) public returns (int64) {
                return 3 + y;
            }
        }"#,
    );

    vm.constructor("bar1", &[], 0);

    let res = vm.function("test_bar", &[Token::Int(U256::from(21))], &[], 0, None);

    assert_eq!(res, vec![Token::Int(U256::from(24))]);

    let bar1_account = vm.stack[0].data;

    vm.set_program(0);

    vm.constructor("bar0", &[], 0);

    let res = vm.function(
        "test_other",
        &[Token::FixedBytes(bar1_account.to_vec())],
        &[],
        0,
        None,
    );

    assert_eq!(res, vec![Token::Int(U256::from(15))]);
}

#[test]
fn external_raw_call_with_returns() {
    let mut vm = build_solidity(
        r#"
        contract bar0 {
            bytes4 private constant SELECTOR = bytes4(keccak256(bytes('test_bar(int64)')));

            function test_other(bar1 x) public returns (int64) {
                bytes select = abi.encodeWithSelector(SELECTOR, int64(7));
                bytes signature = abi.encodeWithSignature("test_bar(int64)", int64(7));
                require(select == signature, "must be the same");
                (, bytes raw) = address(x).call(signature);
                (int64 v) = abi.decode(raw, (int64));
                return v + 5;
            }
        }

        contract bar1 {
            function test_bar(int64 y) public returns (int64) {
                return 3 + y;
            }
        }"#,
    );

    vm.constructor("bar1", &[], 0);

    let res = vm.function("test_bar", &[Token::Int(U256::from(21))], &[], 0, None);

    assert_eq!(res, vec![Token::Int(U256::from(24))]);

    let bar1_account = vm.stack[0].data;

    vm.set_program(0);

    vm.constructor("bar0", &[], 0);

    let res = vm.function(
        "test_other",
        &[Token::FixedBytes(bar1_account.to_vec())],
        &[],
        0,
        None,
    );

    assert_eq!(res, vec![Token::Int(U256::from(15))]);
}

#[test]
fn call_external_func_type() {
    let mut vm = build_solidity(
        r#"
    contract testing {

    function testPtr(int a) public pure returns (int, int) {
        return (a/2, 3);
    }

    function doTest() public view returns (int, int) {
    function(int) external pure returns (int, int) sfPtr = this.testPtr;

       (int a, int b) = sfPtr(2);
       return (a, b);
    }
}
    "#,
    );

    vm.constructor("testing", &[], 0);

    let res = vm.function("doTest", &[], &[], 0, None);

    assert_eq!(
        res,
        vec![Token::Int(U256::from(1)), Token::Int(U256::from(3))]
    );
}

#[test]
fn external_call_with_string_returns() {
    let mut vm = build_solidity(
        r#"
        contract bar0 {
            function test_other(bar1 x) public returns (string) {
                string y = x.test_bar(7);
                print(y);
                return y;
            }

            function test_this(bar1 x) public {
                address a = x.who_am_i();
                assert(a == address(x));
            }

            function test_sender(bar1 x) public returns (address) {
                return x.who_is_sender();
            }
        }

        contract bar1 {
            function test_bar(int64 y) public returns (string) {
                return "foo:{}".format(y);
            }

            function who_am_i() public returns (address) {
                return address(this);
            }

            function who_is_sender() public returns (address) {
                return msg.sender;
            }
        }"#,
    );

    vm.constructor("bar1", &[], 0);

    let res = vm.function("test_bar", &[Token::Int(U256::from(22))], &[], 0, None);

    assert_eq!(res, vec![Token::String(String::from("foo:22"))]);

    let bar1_account = vm.stack[0].data;

    vm.set_program(0);

    vm.constructor("bar0", &[], 0);

    let bar0_account = vm.stack[0].data;

    let res = vm.function(
        "test_other",
        &[Token::FixedBytes(bar1_account.to_vec())],
        &[],
        0,
        None,
    );

    assert_eq!(res, vec![Token::String(String::from("foo:7"))]);

    vm.function(
        "test_this",
        &[Token::FixedBytes(bar1_account.to_vec())],
        &[],
        0,
        None,
    );

    let res = vm.function(
        "test_sender",
        &[Token::FixedBytes(bar1_account.to_vec())],
        &[],
        0,
        None,
    );

    assert_eq!(res[0], Token::FixedBytes(bar0_account.to_vec()));
}

#[test]
fn encode_call() {
    let mut vm = build_solidity(
        r#"
        contract bar0 {
            bytes4 private constant SELECTOR = bytes4(keccak256(bytes('test_bar(int64)')));

            function test_other(bar1 x) public returns (int64) {
                bytes select = abi.encodeWithSelector(SELECTOR, int64(7));
                bytes signature = abi.encodeCall(bar1.test_bar, 7);
                require(select == signature, "must be the same");
                (, bytes raw) = address(x).call(signature);
                (int64 v) = abi.decode(raw, (int64));
                return v + 5;
            }
        }

        contract bar1 {
            function test_bar(int64 y) public returns (int64) {
                return 3 + y;
            }
        }"#,
    );

    vm.constructor("bar1", &[], 0);

    let res = vm.function("test_bar", &[Token::Int(U256::from(21))], &[], 0, None);

    assert_eq!(res, vec![Token::Int(U256::from(24))]);

    let bar1_account = vm.stack[0].data;

    vm.set_program(0);

    vm.constructor("bar0", &[], 0);

    let res = vm.function(
        "test_other",
        &[Token::FixedBytes(bar1_account.to_vec())],
        &[],
        0,
        None,
    );

    assert_eq!(res, vec![Token::Int(U256::from(15))]);
}

#[test]
fn internal_function_storage() {
    let mut vm = build_solidity(
        r#"
        contract ft {
            function(int32,int32) internal returns (int32) func;

            function mul(int32 a, int32 b) internal returns (int32) {
                return a * b;
            }

            function add(int32 a, int32 b) internal returns (int32) {
                return a + b;
            }

            function set_op(bool action) public {
                if (action) {
                    func = mul;
                } else {
                    func = add;
                }
            }

            function test(int32 a, int32 b) public returns (int32) {
                return func(a, b);
            }
        }"#,
    );

    vm.constructor("ft", &[], 0);

    let res = vm.function("set_op", &[Token::Bool(true)], &[], 0, None);

    assert_eq!(res, vec![]);

    let res = vm.function(
        "test",
        &[Token::Int(U256::from(3)), Token::Int(U256::from(5))],
        &[],
        0,
        None,
    );

    assert_eq!(res, vec![Token::Int(U256::from(15))]);

    let res = vm.function("set_op", &[Token::Bool(false)], &[], 0, None);

    assert_eq!(res, vec![]);

    let res = vm.function(
        "test",
        &[Token::Int(U256::from(3)), Token::Int(U256::from(5))],
        &[],
        0,
        None,
    );

    assert_eq!(res, vec![Token::Int(U256::from(8))]);
}

#[test]
fn raw_call_accounts() {
    let mut vm = build_solidity(
        r#"
        import {AccountMeta} from 'solana';

        contract SplToken {
            address constant tokenProgramId = address"TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
            address constant SYSVAR_RENT_PUBKEY = address"SysvarRent111111111111111111111111111111111";

            struct InitializeMintInstruction {
                uint8 instruction;
                uint8 decimals;
                address mintAuthority;
                uint8 freezeAuthorityOption;
                address freezeAuthority;
            }

            function create_mint_with_freezeauthority(uint8 decimals, address mintAuthority, address freezeAuthority) public {
                InitializeMintInstruction instr = InitializeMintInstruction({
                    instruction: 0,
                    decimals: decimals,
                    mintAuthority: mintAuthority,
                    freezeAuthorityOption: 1,
                    freezeAuthority: freezeAuthority
                });

                AccountMeta[2] metas = [
                    AccountMeta({pubkey: instr.mintAuthority, is_writable: true, is_signer: false}),
                    AccountMeta({pubkey: SYSVAR_RENT_PUBKEY, is_writable: false, is_signer: false})
                ];

                tokenProgramId.call{accounts: metas}(instr);
            }
        }"#,
    );

    vm.constructor("SplToken", &[], 0);

    let token = Pubkey(
        "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
            .from_base58()
            .unwrap()
            .try_into()
            .unwrap(),
    );

    let test_args = |instr: &Instruction| {
        let sysvar_rent = Pubkey(
            "SysvarRent111111111111111111111111111111111"
                .from_base58()
                .unwrap()
                .try_into()
                .unwrap(),
        );

        assert_eq!(
            &instr.data,
            &[
                0, 11, 113, 117, 105, 110, 113, 117, 97, 103, 105, 110, 116, 97, 113, 117, 97, 100,
                114, 105, 110, 103, 101, 110, 116, 105, 108, 108, 105, 97, 114, 100, 116, 104, 1,
                113, 117, 105, 110, 113, 117, 97, 103, 105, 110, 116, 97, 113, 117, 97, 100, 114,
                105, 110, 103, 101, 110, 116, 105, 108, 108, 105, 111, 110, 116, 104, 115,
            ]
        );

        assert!(instr.accounts[0].is_writable);
        assert!(!instr.accounts[0].is_signer);
        assert_eq!(
            instr.accounts[0].pubkey,
            Pubkey([
                113, 117, 105, 110, 113, 117, 97, 103, 105, 110, 116, 97, 113, 117, 97, 100, 114,
                105, 110, 103, 101, 110, 116, 105, 108, 108, 105, 97, 114, 100, 116, 104
            ])
        );

        assert!(!instr.accounts[1].is_writable);
        assert!(!instr.accounts[1].is_signer);
        assert_eq!(instr.accounts[1].pubkey, sysvar_rent);
    };

    vm.call_test.insert(token, test_args);

    vm.function(
        "create_mint_with_freezeauthority",
        &[
            Token::Uint(U256::from(11)),
            Token::FixedBytes(b"quinquagintaquadringentilliardth".to_vec()),
            Token::FixedBytes(b"quinquagintaquadringentillionths".to_vec()),
        ],
        &[],
        0,
        None,
    );
}
