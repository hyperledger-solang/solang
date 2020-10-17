use parity_scale_codec::Encode;
use parity_scale_codec_derive::{Decode, Encode};

use super::{build_solidity, first_error, first_warning, parse_and_resolve};
use solang::Target;

#[test]
fn abi_decode() {
    let ns = parse_and_resolve(
        r#"
        contract printer {
            function test() public {
                (int a) = abi.decode(hex"00", feh);
            }
        }"#,
        Target::Substrate,
    );

    assert_eq!(first_error(ns.diagnostics), "type ‘feh’ not found");

    let ns = parse_and_resolve(
        r#"
        contract printer {
            function test() public {
                (int a) = abi.decode(hex"00", (int storage));
            }
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "storage modifier ‘storage’ not allowed"
    );

    let ns = parse_and_resolve(
        r#"
        contract printer {
            function test() public {
                (int a) = abi.decode(hex"00", (int feh));
            }
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "unexpected identifier ‘feh’ in type"
    );

    let ns = parse_and_resolve(
        r#"
        contract printer {
            function test() public {
                (int a) = abi.decode(hex"00", (int,));
            }
        }"#,
        Target::Substrate,
    );

    assert_eq!(first_error(ns.diagnostics), "missing type");

    let ns = parse_and_resolve(
        r#"
        contract printer {
            function test() public {
                (int a) = abi.decode(hex"00", (int,mapping(uint[] => address)));
            }
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "key of mapping cannot be array type"
    );

    let mut runtime = build_solidity(
        r##"
        contract bar {
            function test() public {
                (int16 a, bool b) = abi.decode(hex"7f0001", (int16, bool));

                assert(a == 127);
                assert(b == true);
            }
        }"##,
    );

    runtime.function("test", Vec::new());

    let mut runtime = build_solidity(
        r##"
        contract bar {
            function test() public {
                uint8 a = abi.decode(hex"40", (uint8));

                assert(a == 64);
            }
        }"##,
    );

    runtime.function("test", Vec::new());
}

#[test]
fn abi_encode() {
    let mut runtime = build_solidity(
        r##"
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
        }"##,
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
        r##"
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
        }"##,
    );

    runtime.function("test", Vec::new());

    runtime.function("test2", Vec::new());

    runtime.function("test3", Vec::new());
}

#[test]
fn abi_encode_with_selector() {
    let ns = parse_and_resolve(
        r#"
        contract printer {
            function test() public {
                bytes x = abi.encodeWithSelector();
            }
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "function requires one ‘bytes4’ selector argument"
    );

    let mut runtime = build_solidity(
        r##"
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
        }"##,
    );

    runtime.function("test1", Vec::new());

    runtime.function("test2", Vec::new());
}

#[test]
fn abi_encode_with_signature() {
    let ns = parse_and_resolve(
        r#"
        contract printer {
            function test() public {
                bytes x = abi.encodeWithSignature();
            }
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "function requires one ‘string’ signature argument"
    );

    let mut runtime = build_solidity(
        r##"
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
        }"##,
    );

    runtime.constructor(0, Vec::new());
    runtime.function("test1", Vec::new());
    runtime.function("test2", Vec::new());
}

#[test]
fn call() {
    let ns = parse_and_resolve(
        r#"
        contract main {
            function test() public {
                address x = address(0);

                x.delegatecall(hex"1222");
            }
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "method ‘delegatecall’ does not exist"
    );

    let ns = parse_and_resolve(
        r#"
        contract main {
            function test() public {
                address x = address(0);

                x.staticcall(hex"1222");
            }
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "method ‘staticcall’ does not exist"
    );

    let ns = parse_and_resolve(
        r#"
        contract superior {
            function test() public {
                inferior i = new inferior();

                bytes x = address(i).call(hex"1222");
            }
        }

        contract inferior {
            function baa() public {
                print("Baa!");
            }
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "destucturing statement needed for function that returns multiple values"
    );

    let ns = parse_and_resolve(
        r#"
        contract superior {
            function test() public {
                inferior i = new inferior();

            (bytes x, bool y) = address(i).call(hex"1222");
            }
        }

        contract inferior {
            function baa() public {
                print("Baa!");
            }
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "conversion from bool to bytes not possible"
    );

    let mut runtime = build_solidity(
        r##"
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
        }"##,
    );

    runtime.function("test1", Vec::new());
    runtime.function("test2", Vec::new());

    let mut runtime = build_solidity(
        r##"
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
        }"##,
    );

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

    let ns = parse_and_resolve(
        r#"
        contract bar {
            function test() public {
                int64 b = block.number;

                assert(b == 14_250_083_331_950_119_597);
            }
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "implicit conversion would change sign from uint64 to int64"
    );

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

    let ns = parse_and_resolve(
        r#"
        contract bar {
            function test() public {
                int64 b = block.timestamp;

                assert(b == 14_250_083_331_950_119_597);
            }
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "implicit conversion would change sign from uint64 to int64"
    );

    let mut runtime = build_solidity(
        r##"
        contract bar {
            function test() public {
                uint128 b = block.tombstone_deposit;

                assert(b == 93_603_701_976_053);
            }
        }"##,
    );

    runtime.function("test", Vec::new());

    let ns = parse_and_resolve(
        r#"
        contract bar {
            function test() public {
                int64 b = block.tombstone_deposit;

                assert(b == 93_603_701_976_053);
            }
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "implicit conversion would change sign from uint128 to int64"
    );

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

    let ns = parse_and_resolve(
        r#"
        contract bar {
            function test() public {
                int64 b = block.minimum_balance;

                assert(b == 93_603_701_976_053);
            }
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "implicit conversion would change sign from uint128 to int64"
    );

    let ns = parse_and_resolve(
        r#"
        contract bar {
            function test() public {
                int64 b = block.coinbase;

                assert(b == 93_603_701_976_053);
            }
        }"#,
        Target::Substrate,
    );

    assert_eq!(first_error(ns.diagnostics), "`block\' is not found");

    let ns = parse_and_resolve(
        r#"
        contract bar {
            function test() public {
                int64 b = block.gaslimit;

                assert(b == 93_603_701_976_053);
            }
        }"#,
        Target::Substrate,
    );

    assert_eq!(first_error(ns.diagnostics), "`block\' is not found");

    let ns = parse_and_resolve(
        r#"
        contract bar {
            function test() public {
                int64 b = block.difficulty;

                assert(b == 93_603_701_976_053);
            }
        }"#,
        Target::Substrate,
    );

    assert_eq!(first_error(ns.diagnostics), "`block\' is not found");
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

    let ns = parse_and_resolve(
        r#"
        contract bar {
            function test() public {
                int128 b = tx.gasprice;
            }
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "use the function ‘tx.gasprice(gas)’ in stead, as ‘tx.gasprice’ may round down to zero. See https://solang.readthedocs.io/en/latest/language.html#gasprice"
    );

    let ns = parse_and_resolve(
        r#"
        contract bar {
            function test() public {
                int128 b = tx.gasprice(4-3);
            }
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_warning(ns.diagnostics),
        "the function call ‘tx.gasprice(1)’ may round down to zero. See https://solang.readthedocs.io/en/latest/language.html#gasprice"
    );

    let ns = parse_and_resolve(
        r#"
        contract bar {
            function test() public {
                int64 b = tx.gasprice(100);

                assert(b == 14_250_083_331_950_119_597);
            }
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "implicit conversion would change sign from uint128 to int64"
    );

    let ns = parse_and_resolve(
        r#"
        contract bar {
            function test() public {
                int64 b = tx.origin;

                assert(b == 93_603_701_976_053);
            }
        }"#,
        Target::Substrate,
    );

    assert_eq!(first_error(ns.diagnostics), "`tx\' is not found");
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

    runtime.vm.value = 145_594_775_678_703_046_797_448_357_509_034_994_219;
    runtime.function("test", Vec::new());

    let ns = parse_and_resolve(
        r#"
        contract bar {
            function test() public {
                int64 b = msg.value;

                assert(b == 14_250_083_331_950_119_597);
            }
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "implicit conversion would change sign from uint128 to int64"
    );

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

    let ns = parse_and_resolve(
        r#"
        contract bar {
            function test() public {
                int64 b = gasleft();

                assert(b == 14_250_083_331_950_119_597);
            }
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "implicit conversion would change sign from uint64 to int64"
    );

    let ns = parse_and_resolve(
        r#"
        contract bar {
            function test() public {
                int64 b = gasleft(1);

                assert(b == 14_250_083_331_950_119_597);
            }
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "builtin function ‘gasleft’ expects 0 arguments, 1 provided"
    );

    let ns = parse_and_resolve(
        r#"
        contract bar {
            function test() public {
                bytes32 b = blockhash(1);
            }
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "unknown function or type ‘blockhash’"
    );

    let mut runtime = build_solidity(
        r##"
        contract c {
            function test() public {
                bytes32 o = random(
                    "abcd"
                );

                assert(o == hex"429ccf3ebce07f0c6d7cd0d1dead74459f753cdf53ed8359e42728042a91c39c");
            }
        }"##,
    );

    runtime.function("test", Vec::new());
}

#[test]
fn data() {
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct Uint32(u32);
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct String(Vec<u8>);

    let mut runtime = build_solidity(
        r##"
        contract bar {
            constructor(string memory s) public {
                assert(msg.data == hex"88eaeb6c18666f6f626172");
                assert(msg.sig == hex"88ea_eb6c");
            }

            function test(uint32 x) public {
                assert(msg.data == hex"e3cff634addeadde");
                assert(msg.sig == hex"e3cf_f634");
            }
        }"##,
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
                assert(addmod(500, 100, 3) == 200);
            }
        }"##,
    );

    runtime.function("test", Vec::new());

    // divide by zero
    let mut runtime = build_solidity(
        r##"
        contract x {
            function test() public {
                assert(addmod(500, 100, 0) == 200);
            }
        }"##,
    );

    runtime.function_expect_return("test", Vec::new(), 1);

    // bigger numbers (64 bit)
    let mut runtime = build_solidity(
        r##"
        contract x {
            function test() public {
                // 8_163_321_534_310_945_187 * 16_473_784_705_703_234_153 = 134_480_801_439_669_508_040_541_782_812_209_371_611
                assert(addmod(
                    0,
                    134_480_801_439_669_508_040_541_782_812_209_371_611,
                    16_473_784_705_703_234_153) == 8_163_321_534_310_945_187);
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
                    148_872_967_607_295_528_830_315_866_466_318_446_379) == 254_765_928_331_839_140_628_748_569_208_536_440_801);
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
                    233) == 681774308917621516739871418731032629545104964623958032502757716208566275960);
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
                    2) == 109802613191917590715814365746623394364442484359636492253827647701845853490667);
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
                assert(mulmod(500, 100, 5) == 10000);
            }
        }"##,
    );

    runtime.function("test", Vec::new());

    // divide by zero
    let mut runtime = build_solidity(
        r##"
        contract x {
            function test() public {
                assert(mulmod(500, 100, 0) == 200);
            }
        }"##,
    );

    runtime.function_expect_return("test", Vec::new(), 1);

    // bigger numbers
    let mut runtime = build_solidity(
        r##"
        contract x {
            function test() public {
                assert(mulmod(50000, 10000, 5) == 10000_0000);
            }
        }"##,
    );

    runtime.function("test", Vec::new());

    let mut runtime = build_solidity(
        r##"
        contract x {
            function test() public {
                assert(mulmod(18446744073709551616, 18446744073709550403, 1024) == 332306998946228946374486373068439552);
            }
        }"##,
    );

    runtime.function("test", Vec::new());

    // 2^127 = 170141183460469231731687303715884105728
    let mut runtime = build_solidity(
        r##"
        contract x {
            function test() public {
                assert(mulmod(170141183460469231731687303715884105728, 170141183460469231731687303715884105728, 170141183460469231731687303715884105728) == 170141183460469231731687303715884105728);
            }
        }"##,
    );

    runtime.function("test", Vec::new());

    // 2^128 = 340282366920938463463374607431768211456
    let mut runtime = build_solidity(
        r##"
        contract x {
            function test() public {
                assert(mulmod(340282366920938463463374607431768211456, 340282366920938463463374607431768211456, 340282366920938463463374607431768211456) == 340282366920938463463374607431768211456);
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
                    == 1766847064778384329583297500742918515827483896875618958121606201292619776);
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
                    == 824364134751099588297822369420176791913922347811791536817152126684405253);
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
                    == 113477814626329405513123655892059150026234290706112418221315641434319827527851);
            }
        }"##,
    );

    runtime.function("test", Vec::new());
}
