// SPDX-License-Identifier: Apache-2.0

use parity_scale_codec::{Decode, Encode};

use crate::build_solidity;

#[test]
fn abi_decode() {
    let mut runtime = build_solidity(
        r#"
        contract bar {
            function test() public {
                (int16 a, bool b) = abi.decode(hex"7f0001", (int16, bool));

                assert(a == 127);
                assert(b == true);
            }
        }"#,
    );

    runtime.function("test", Vec::new());

    let mut runtime = build_solidity(
        r#"
        contract bar {
            function test() public {
                uint8 a = abi.decode(hex"40", (uint8));

                assert(a == 64);
            }
        }"#,
    );

    runtime.function("test", Vec::new());
}

#[test]
fn abi_encode() {
    let mut runtime = build_solidity(
        r#"
        struct s {
            int32 f1;
            uint8 f2;
            string f3;
            uint16[2] f4;
        }

        contract bar {
            function test() public {
                uint16 a = 0xfd01;
                assert(abi.encode(a) == hex"01fd");
                uint32 b = 0xaabbccdd;
                assert(abi.encode(true, b, false) == hex"01ddccbbaa00");
            }

            function test2() public {
                string b = "foobar";
                assert(abi.encode(b) == hex"18666f6f626172");

                assert(abi.encode("foobar") == hex"18666f6f626172");
            }

            function test3() public {
                s x = s({ f1: 511, f2: 0xf7, f3: "testie", f4: [ uint16(4), 5 ] });

                assert(abi.encode(x) == hex"ff010000f71874657374696504000500");
            }
        }"#,
    );

    runtime.function("test", Vec::new());
    runtime.heap_verify();

    runtime.function("test2", Vec::new());
    runtime.heap_verify();

    runtime.function("test3", Vec::new());
    runtime.heap_verify();
}

#[test]
fn abi_encode_packed() {
    let mut runtime = build_solidity(
        r#"
        struct s {
            int32 f1;
            uint8 f2;
            string f3;
            uint16[2] f4;
        }

        contract bar {
            function test() public {
                uint16 a = 0xfd01;
                assert(abi.encodePacked(a) == hex"01fd");
                uint32 b = 0xaabbccdd;
                assert(abi.encodePacked(true, b, false) == hex"01ddccbbaa00");
            }

            function test2() public {
                string b = "foobar";
                assert(abi.encodePacked(b) == "foobar");

                assert(abi.encodePacked("foobar") == "foobar");
                assert(abi.encodePacked("foo", "bar") == "foobar");
            }

            function test3() public {
                s x = s({ f1: 511, f2: 0xf7, f3: "testie", f4: [ uint16(4), 5 ] });

                assert(abi.encodePacked(x) == hex"ff010000f774657374696504000500");
            }
        }"#,
    );

    runtime.function("test", Vec::new());

    runtime.function("test2", Vec::new());

    runtime.function("test3", Vec::new());
}

#[test]
fn abi_encode_with_selector() {
    let mut runtime = build_solidity(
        r#"
        contract bar {
            function test1() public {
                uint16 a = 0xfd01;
                assert(abi.encodeWithSelector(hex"44332211", a) == hex"4433221101fd");
                uint32 b = 0xaabbccdd;
                assert(abi.encodeWithSelector(hex"aabbccdd", true, b, false) == hex"aabbccdd01ddccbbaa00");

                assert(abi.encodeWithSelector(hex"aabbccdd") == hex"aabbccdd");
            }

            function test2() public {
                uint8[] arr = new uint8[](3);

                arr[0] = 0xfe;
                arr[1] = 0xfc;
                arr[2] = 0xf8;

                assert(abi.encodeWithSelector(hex"01020304", arr) == hex"010203040cfefcf8");
            }
        }"#,
    );

    runtime.function("test1", Vec::new());

    runtime.function("test2", Vec::new());
}

#[test]
fn abi_encode_with_signature() {
    let mut runtime = build_solidity(
        r#"
        contract bar {
            string bla = "Hello, World!";

            function test1() public {
                assert(keccak256("Hello, World!") == hex"acaf3289d7b601cbd114fb36c4d29c85bbfd5e133f14cb355c3fd8d99367964f");

                assert(abi.encodeWithSignature("Hello, World!") == hex"acaf3289");
                assert(abi.encodeWithSignature(bla) == hex"acaf3289");
            }

            function test2() public {
                uint8[] arr = new uint8[](3);

                arr[0] = 0xfe;
                arr[1] = 0xfc;
                arr[2] = 0xf8;

                assert(abi.encodeWithSelector(hex"01020304", arr) == hex"010203040cfefcf8");
            }
        }"#,
    );

    runtime.constructor(0, Vec::new());
    runtime.function("test1", Vec::new());
    runtime.function("test2", Vec::new());
}

#[test]
fn call() {
    let mut runtime = build_solidity(
        r#"
        contract superior {
            function test1() public {
                inferior i = new inferior();

                i.test1();

                assert(keccak256("test1()") == hex"6b59084dfb7dcf1c687dd12ad5778be120c9121b21ef90a32ff73565a36c9cd3");

                bytes bs;
                bool success;

                (success, bs) = address(i).call(hex"6b59084d");

                assert(success == true);
                assert(bs == hex"");
            }

            function test2() public {
                inferior i = new inferior();

                assert(i.test2(257) == 256);

                assert(keccak256("test2(uint64)") == hex"296dacf0801def8823747fbd751fbc1444af573e88de40d29c4d01f6013bf095");

                bytes bs;
                bool success;

                (success, bs) = address(i).call(hex"296dacf0_0101_0000__0000_0000");

                assert(success == true);
                assert(bs == hex"0001_0000__0000_0000");
            }
        }

        contract inferior {
            function test1() public {
                print("Baa!");
            }

            function test2(uint64 x) public returns (uint64) {
                return x ^ 1;
            }
        }"#,
    );

    runtime.constructor(0, Vec::new());
    runtime.function("test1", Vec::new());
    runtime.function("test2", Vec::new());

    let mut runtime = build_solidity(
        r#"
        contract superior {
            function test1() public {
                inferior i = new inferior();

                assert(keccak256("test1()") == hex"6b59084dfb7dcf1c687dd12ad5778be120c9121b21ef90a32ff73565a36c9cd3");

                bytes bs;
                bool success;

                (success, bs) = address(i).call(abi.encodeWithSelector(hex"6b59084d"));

                assert(success == true);
                assert(bs == hex"");

                (success, bs) = address(i).call(abi.encodeWithSignature("test1()"));

                assert(success == true);
                assert(bs == hex"");
            }

            function test2() public {
                inferior i = new inferior();
                assert(keccak256("test2(uint64)") == hex"296dacf0801def8823747fbd751fbc1444af573e88de40d29c4d01f6013bf095");

                bytes bs;
                bool success;

                (success, bs) = address(i).call(abi.encodeWithSelector(hex"296dacf0", uint64(257)));

                assert(success == true);

                assert(abi.decode(bs, (uint64)) == 256);


                (success, bs) = address(i).call(abi.encodeWithSignature("test2(uint64)", uint64(0xfeec)));

                assert(success == true);

                assert(abi.decode(bs, (uint64)) == 0xfeed);
            }
        }

        contract inferior {
            function test1() public {
                print("Baa!");
            }

            function test2(uint64 x) public returns (uint64) {
                return x ^ 1;
            }
        }"#,
    );

    runtime.constructor(0, Vec::new());
    runtime.function("test1", Vec::new());
    runtime.function("test2", Vec::new());
}

#[test]
fn block() {
    let mut runtime = build_solidity(
        r##"
        contract bar {
            function test() public {
                uint64 b = block.number;

                assert(b == 950_119_597);
            }
        }"##,
    );

    runtime.function("test", Vec::new());

    let mut runtime = build_solidity(
        r##"
        contract bar {
            function test() public {
                uint64 b = block.timestamp;

                assert(b == 1594035638);
            }
        }"##,
    );

    runtime.function("test", Vec::new());

    let mut runtime = build_solidity(
        r##"
        contract bar {
            function test() public {
                uint128 b = block.minimum_balance;

                assert(b == 500);
            }
        }"##,
    );

    runtime.function("test", Vec::new());
}

#[test]
fn tx() {
    let mut runtime = build_solidity(
        r##"
        contract bar {
            function test() public {
                uint128 b = tx.gasprice(1);

                assert(b == 59_541_253_813_967);
            }
        }"##,
    );

    runtime.function("test", Vec::new());

    let mut runtime = build_solidity(
        r##"
        contract bar {
            function test() public {
                uint128 b = tx.gasprice(1000);

                assert(b == 59_541_253_813_967_000);
            }
        }"##,
    );

    runtime.function("test", Vec::new());
}

#[test]
fn msg() {
    let mut runtime = build_solidity(
        r##"
        contract bar {
            function test() public payable {
                uint128 b = msg.value;

                assert(b == 145_594_775_678_703_046_797_448_357_509_034_994_219);
            }
        }"##,
    );

    let value = 145_594_775_678_703_046_797_448_357_509_034_994_219;
    runtime.set_transferred_value(value);
    runtime.raw_function(runtime.contracts()[0].code.messages["test"].to_vec());

    let mut runtime = build_solidity(
        r##"
        contract c {
            function test() public {
                other o = new other();
                address foo = o.test();

                assert(foo == address(this));
            }
        }

        contract other {
            function test() public returns (address) {
                return msg.sender;
            }
        }
        "##,
    );

    runtime.constructor(0, Vec::new());
    runtime.function("test", Vec::new());
}

#[test]
fn functions() {
    let mut runtime = build_solidity(
        r##"
        contract bar {
            function test() public {
                uint64 b = gasleft();

                assert(b == 2_224_097_461);
            }
        }"##,
    );

    runtime.function("test", Vec::new());
}

#[test]
fn data() {
    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    struct Uint32(u32);
    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    struct String(Vec<u8>);

    let mut runtime = build_solidity(
        r#"
        contract bar {
            constructor(string memory s) public {
                assert(msg.data == hex"98dd1bb318666f6f626172");
                assert(msg.sig == hex"98dd_1bb3");
            }

            function test(uint32 x) public {
                assert(msg.data == hex"e3cff634addeadde");
                assert(msg.sig == hex"e3cf_f634");
            }
        }"#,
    );

    runtime.constructor(0, String(b"foobar".to_vec()).encode());
    runtime.function("test", Uint32(0xdeaddead).encode());
}

#[test]
fn addmod() {
    // does it work with small numbers
    let mut runtime = build_solidity(
        r##"
        contract x {
            function test() public {
                assert(addmod(500, 100, 3) == 0);
            }
        }"##,
    );

    runtime.function("test", Vec::new());

    // divide by zero
    let mut runtime = build_solidity(
        r##"
        contract x {
            function test() public {
                assert(addmod(500, 100, 0) == 0);
            }
        }"##,
    );

    runtime.function("test", Vec::new());

    // bigger numbers (64 bit)
    let mut runtime = build_solidity(
        r##"
        contract x {
            function test() public {
                // 8_163_321_534_310_945_187 * 16_473_784_705_703_234_153 = 134_480_801_439_669_508_040_541_782_812_209_371_611
                assert(addmod(
                    0,
                    134_480_801_439_669_508_040_541_782_812_209_371_611,
                    16_473_784_705_703_234_153) == 0);
            }
        }"##,
    );

    runtime.function("test", Vec::new());

    // bigger numbers (128 bit)
    let mut runtime = build_solidity(
        r##"
        contract x {
            function test() public {
                // 254_765_928_331_839_140_628_748_569_208_536_440_801 * 148_872_967_607_295_528_830_315_866_466_318_446_379 = 37_927_759_795_988_462_606_362_647_643_228_779_300_269_446_446_871_437_380_583_919_404_728_626_309_579
                assert(addmod(
                    0,
                    37_927_759_795_988_462_606_362_647_643_228_779_300_269_446_446_871_437_380_583_919_404_728_626_309_579,
                    148_872_967_607_295_528_830_315_866_466_318_446_379) == 0);
            }
        }"##,
    );

    runtime.function("test", Vec::new());

    // bigger numbers (256 bit)
    let mut runtime = build_solidity(
        r##"
        contract x {
            function test() public {
                assert(addmod(
                    109802613191917590715814365746623394364442484359636492253827647701845853490667,
                    49050800785888222684575674817707208319566972397745729319314900174750088808217,
                    233) == 204);
            }
        }"##,
    );

    runtime.function("test", Vec::new());

    let mut runtime = build_solidity(
        r##"
        contract x {
            function test() public {
                assert(addmod(
                    109802613191917590715814365746623394364442484359636492253827647701845853490667,
                    109802613191917590715814365746623394364442484359636492253827647701845853490667,
                    2) == 0);
            }
        }"##,
    );

    runtime.function("test", Vec::new());
}

#[test]
fn mulmod() {
    // does it work with small numbers
    let mut runtime = build_solidity(
        r##"
        contract x {
            function test() public {
                assert(mulmod(500, 100, 5) == 0);
            }
        }"##,
    );

    runtime.function("test", Vec::new());

    // divide by zero
    let mut runtime = build_solidity(
        r##"
        contract x {
            function test() public {
                assert(mulmod(500, 100, 0) == 0);
            }
        }"##,
    );

    runtime.function("test", Vec::new());

    // bigger numbers
    let mut runtime = build_solidity(
        r##"
        contract x {
            function test() public {
                assert(mulmod(50000, 10000, 5) == 0);
            }
        }"##,
    );

    runtime.function("test", Vec::new());

    let mut runtime = build_solidity(
        r##"
        contract x {
            function test() public {
                assert(mulmod(18446744073709551616, 18446744073709550403, 1024) == 0);
            }
        }"##,
    );

    runtime.function("test", Vec::new());

    // 2^127 = 170141183460469231731687303715884105728
    let mut runtime = build_solidity(
        r##"
        contract x {
            function test() public {
                assert(mulmod(170141183460469231731687303715884105728, 170141183460469231731687303715884105728, 170141183460469231731687303715884105728) == 0);
            }
        }"##,
    );

    runtime.function("test", Vec::new());

    // 2^128 = 340282366920938463463374607431768211456
    let mut runtime = build_solidity(
        r##"
        contract x {
            function test() public {
                assert(mulmod(340282366920938463463374607431768211456, 340282366920938463463374607431768211456, 340282366920938463463374607431768211456) == 0);
            }
        }"##,
    );

    runtime.function("test", Vec::new());

    // 2^240 = 1766847064778384329583297500742918515827483896875618958121606201292619776
    let mut runtime = build_solidity(
        r##"
        contract x {
            function test() public {
                assert(mulmod(1766847064778384329583297500742918515827483896875618958121606201292619776,
                    1766847064778384329583297500742918515827483896875618958121606201292619776,
                    1766847064778384329583297500742918515827483896875618958121606201292619776)
                    == 0);
            }
        }"##,
    );

    runtime.function("test", Vec::new());

    // 240 bit prime: 824364134751099588297822369420176791913922347811791536817152126684405253
    let mut runtime = build_solidity(
        r##"
        contract x {
            function test() public {
                assert(mulmod(824364134751099588297822369420176791913922347811791536817152126684405253,
                    824364134751099588297822369420176791913922347811791536817152126684405253,
                    824364134751099588297822369420176791913922347811791536817152126684405253)
                    == 0);
            }
        }"##,
    );

    runtime.function("test", Vec::new());

    // 256 bit prime: 113477814626329405513123655892059150026234290706112418221315641434319827527851
    let mut runtime = build_solidity(
        r##"
        contract x {
            function test() public {
                assert(mulmod(113477814626329405513123655892059150026234290706112418221315641434319827527851,
                    113477814626329405513123655892059150026234290706112418221315641434319827527851,
                    113477814626329405513123655892059150026234290706112418221315641434319827527851)
                    == 0);
            }
        }"##,
    );

    runtime.function("test", Vec::new());

    let mut runtime = build_solidity(
        r##"
        contract x {
            function test() public {
                assert(mulmod(113477814626329405513123655892059150026234290706112418221315641434319827527851,
                    113477814626329405513123655892059150026234290706112418221315641434319827527841,
                    233)
                    == 12);
            }
        }"##,
    );

    runtime.function("test", Vec::new());
}

#[test]
fn my_token() {
    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    struct TokenTest([u8; 32], bool);
    let mut runtime = build_solidity(
        "
        contract mytoken {
            function test(address account, bool sender) public view returns (address) {
                if (sender) {
                    return msg.sender;
                }
                return account;
            }
        }
        ",
    );

    let addr: [u8; 32] = [
        0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15,
        0x16, 0x17, 0x18, 0x19, 0x20, 0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27, 0x28, 0x29, 0x30,
        0x31, 0x32,
    ];
    runtime.function("test", TokenTest(addr, true).encode());
    assert_eq!(&runtime.caller()[..], &runtime.output()[..]);

    runtime.function("test", TokenTest(addr, false).encode());
    assert_eq!(&runtime.output()[..], &addr[..]);

    runtime.function(
        "test",
        TokenTest(<[u8; 32]>::try_from(&runtime.caller()[..]).unwrap(), true).encode(),
    );
    assert_eq!(&runtime.caller()[..], &runtime.output()[..]);

    runtime.function(
        "test",
        TokenTest(<[u8; 32]>::try_from(&runtime.caller()[..]).unwrap(), false).encode(),
    );
    assert_eq!(&runtime.caller()[..], &runtime.output()[..]);
}

#[test]
fn hash() {
    let mut runtime = build_solidity(
        r#"
        import "polkadot";

        contract Foo {
            Hash current;
            bytes32 current2;

            function set(Hash h) public returns (bytes32) {
                current = h;
                current2 = Hash.unwrap(h);
                return current2;
            }

            function get() public view returns (Hash) {
                return Hash.wrap(current2);
            }

            function test_encoding() public view {
                Hash h = Hash.wrap(current2);
                assert(abi.encode(current2) == abi.encode(h));
            }
        }
        "#,
    );

    #[derive(Encode)]
    struct Hash([u8; 32]);

    let h = Hash([
        0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15,
        0x16, 0x17, 0x18, 0x19, 0x20, 0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27, 0x28, 0x29, 0x30,
        0x31, 0x32,
    ]);

    runtime.function("set", h.encode());
    assert_eq!(&runtime.output()[..], &h.0[..]);

    runtime.function("get", vec![]);
    assert_eq!(&runtime.output()[..], &h.0[..]);

    runtime.function("test_encoding", vec![]);
}

#[test]
fn call_chain_extension() {
    let mut runtime = build_solidity(
        r#"
        import {chain_extension as ChainExtension} from "polkadot";

        contract Foo {
            function chain_extension(bytes input) public returns (uint32, bytes) {
                return ChainExtension(123, input);
            }
        }"#,
    );

    let data = 0xdeadbeefu32.to_be_bytes().to_vec();
    runtime.function("chain_extension", data.encode());
    let ret = <(u32, Vec<u8>)>::decode(&mut &runtime.output()[..]).unwrap();
    assert_eq!(ret.0, data.iter().map(|i| *i as u32).sum::<u32>());
    assert_eq!(ret.1, data.iter().cloned().rev().collect::<Vec<_>>());
}

#[test]
fn is_contract() {
    let mut runtime = build_solidity(
        r#"
        import "polkadot";
        contract Foo {
            function test(address _a) public view returns (bool) {
                return is_contract(_a);
            }
        }"#,
    );

    runtime.function("test", runtime.0.data().accounts[0].address.to_vec());
    assert_eq!(runtime.output(), vec![1]);

    runtime.function("test", [0; 32].to_vec());
    assert_eq!(runtime.output(), vec![0]);
}

#[test]
fn set_code_hash() {
    let mut runtime = build_solidity(
        r#"
        import "polkadot";

        abstract contract SetCode {
            function set_code(uint8[32] code_hash) external {
                require(set_code_hash(code_hash) == 0);
            }
        }
        
        contract CounterV1 is SetCode {
            uint32 public count;
        
            function inc() external {
                count += 1;
            }
        }
        
        contract CounterV2 is SetCode {
            uint32 public count;
        
            function inc() external {
                count -= 1;
            }
        }"#,
    );

    runtime.function("inc", vec![]);
    runtime.function("count", vec![]);
    assert_eq!(runtime.output(), 1u32.encode());

    let v2_code_hash = ink_primitives::Hash::default().as_ref().to_vec();
    runtime.function_expect_failure("set_code", v2_code_hash);

    let v2_code_hash = runtime.blobs()[1].hash;
    runtime.function("set_code", v2_code_hash.as_ref().to_vec());

    runtime.function("inc", vec![]);
    runtime.function("count", vec![]);
    assert_eq!(runtime.output(), 0u32.encode());

    let v1_code_hash = runtime.blobs()[0].hash;
    runtime.function("set_code", v1_code_hash.as_ref().to_vec());

    runtime.function("inc", vec![]);
    runtime.function("count", vec![]);
    assert_eq!(runtime.output(), 1u32.encode());
}

#[test]
fn caller_is_root() {
    let mut runtime = build_solidity(
        r#"
        import { caller_is_root } from "polkadot";
        contract Test {
            function test() public view returns (bool) {
                return caller_is_root();
            }
        }"#,
    );

    runtime.function("test", runtime.0.data().accounts[0].address.to_vec());
    assert_eq!(runtime.output(), false.encode());

    // Set the caller address to [0; 32] which is the mock VM root account
    runtime.set_account_address(0, [0; 32]);
    runtime.function("test", [0; 32].to_vec());
    assert_eq!(runtime.output(), true.encode());
}
