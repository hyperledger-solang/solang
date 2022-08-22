// SPDX-License-Identifier: Apache-2.0

use crate::build_solidity;
use crate::solana_tests::abi_encode::create_response;
use borsh::BorshSerialize;
use ethabi::Token;

#[test]
fn integers_bool_enum() {
    #[derive(BorshSerialize, PartialEq, Eq, Debug)]
    #[allow(unused)]
    enum WeekDay {
        Sunday,
        Monday,
        Tuesday,
        Wednesday,
        Thursday,
        Friday,
        Saturday,
    }

    #[derive(BorshSerialize, Debug)]
    struct Res1 {
        a: u8,
        b: u64,
        c: u128,
        d: i16,
        e: i32,
        day: WeekDay,
        h: bool,
    }

    #[derive(BorshSerialize, Debug)]
    struct Res2 {
        item_1: WeekDay,
        item_2: WeekDay,
        item_3: WeekDay,
    }

    let mut vm = build_solidity(
        r#"
    contract Testing {
        enum WeekDay {
            Sunday, Monday, Tuesday, Wednesday, Thursday, Friday, Saturday
        }

        function decodeTest1(bytes memory buffer) public pure {
            (uint8 a, uint64 b, uint128 c, int16 d, int32 e, WeekDay day, bool h) =
            abi.borshDecode(buffer, (uint8, uint64, uint128, int16, int32, WeekDay, bool));

            assert(a == 45);
            assert(b == 9965956609890);
            assert(c == 88);
            assert(d == -29);
            assert(e == -88);
            assert(day == WeekDay.Wednesday);
            assert(h == false);
        }

        function decodeTest2(bytes memory buffer) public pure {
            (WeekDay a, WeekDay b, WeekDay c) =
            abi.borshDecode(buffer, (WeekDay, WeekDay, WeekDay));
            assert(a == WeekDay.Sunday);
            assert(b == WeekDay.Saturday);
            assert(c == WeekDay.Friday);
        }
    }
        "#,
    );

    vm.constructor("Testing", &[]);
    let input = Res1 {
        a: 45,
        b: 9965956609890,
        c: 88,
        d: -29,
        e: -88,
        day: WeekDay::Wednesday,
        h: false,
    };
    let encoded = input.try_to_vec().unwrap();
    let _ = vm.function("decodeTest1", &[Token::Bytes(encoded)], &[], None);

    let input = Res2 {
        item_1: WeekDay::Sunday,
        item_2: WeekDay::Saturday,
        item_3: WeekDay::Friday,
    };
    let encoded = input.try_to_vec().unwrap();
    let _ = vm.function("decodeTest2", &[Token::Bytes(encoded)], &[], None);
}

#[test]
fn decode_address() {
    #[derive(BorshSerialize, Debug)]
    struct Data {
        address: [u8; 32],
        this: [u8; 32],
    }

    let mut vm = build_solidity(
        r#"
    contract Testing {
        function testAddress(bytes memory buffer) public view {
            (address a, Testing b) = abi.borshDecode(buffer, (address, Testing));

            assert(a == address(this));
            assert(b == this);
        }
    }
        "#,
    );

    vm.constructor("Testing", &[]);
    let input = Data {
        address: vm.programs[0].data,
        this: vm.programs[0].data,
    };
    let encoded = input.try_to_vec().unwrap();
    let _ = vm.function("testAddress", &[Token::Bytes(encoded)], &[], None);
}

#[test]
fn string_and_bytes() {
    #[derive(BorshSerialize, Debug)]
    struct Data {
        a: String,
        b: Vec<u8>,
    }

    let mut vm = build_solidity(
        r#"
    contract Testing {
        function testStringAndBytes(bytes memory buffer) public view {
            (string memory a, bytes memory b) = abi.borshDecode(buffer, (string, bytes));

            assert(a == "coffee");
            assert(b == "tea");
        }
    }
        "#,
    );

    vm.constructor("Testing", &[]);
    let data = Data {
        a: "coffee".to_string(),
        b: b"tea".to_vec(),
    };
    let encoded = data.try_to_vec().unwrap();
    let _ = vm.function("testStringAndBytes", &[Token::Bytes(encoded)], &[], None);
}

#[test]
fn primitive_struct() {
    #[derive(Debug, BorshSerialize)]
    struct NoPadStruct {
        a: u32,
        b: u32,
    }

    #[derive(Debug, BorshSerialize)]
    struct PaddedStruct {
        a: u128,
        b: u8,
        c: [u8; 32],
    }

    let mut vm = build_solidity(
        r#"
    contract Testing {
        struct NoPadStruct {
            uint32 a;
            uint32 b;
        }

        struct PaddedStruct {
            uint128 a;
            uint8 b;
            bytes32 c;
        }

        function testNoPadStruct(bytes memory buffer) public pure {
            NoPadStruct memory str = abi.borshDecode(buffer, (NoPadStruct));
            assert(str.a == 1238);
            assert(str.b == 87123);
        }

        function testPaddedStruct(bytes memory buffer) public pure {
            PaddedStruct memory str = abi.borshDecode(buffer, (PaddedStruct));
            assert(str.a == 12998);
            assert(str.b == 240);
            assert(str.c == "tea_is_good");
        }
    }
        "#,
    );

    vm.constructor("Testing", &[]);
    let data = NoPadStruct { a: 1238, b: 87123 };
    let encoded = data.try_to_vec().unwrap();
    let _ = vm.function("testNoPadStruct", &[Token::Bytes(encoded)], &[], None);

    let mut elem = b"tea_is_good".to_vec();
    elem.append(&mut vec![0; 21]);
    let data = PaddedStruct {
        a: 12998,
        b: 240,
        c: <[u8; 32]>::try_from(&elem[0..32]).unwrap(),
    };
    let encoded = data.try_to_vec().unwrap();
    let _ = vm.function("testPaddedStruct", &[Token::Bytes(encoded)], &[], None);
}

#[test]
fn returned_string() {
    #[derive(Debug, BorshSerialize)]
    struct Input {
        rr: String,
    }

    let mut vm = build_solidity(
        r#"
    contract Testing {
           function returnedString(bytes memory buffer) public pure returns (string memory) {
                string memory s = abi.borshDecode(buffer, (string));
                return s;
           }
    }
        "#,
    );
    vm.constructor("Testing", &[]);
    let data = Input {
        rr: "cortado".to_string(),
    };
    let encoded = data.try_to_vec().unwrap();
    let returns = vm.function("returnedString", &[Token::Bytes(encoded)], &[], None);
    let string = returns[0].clone().into_string().unwrap();
    assert_eq!(string, "cortado");
}

#[test]
fn test_string_array() {
    #[derive(Debug, BorshSerialize)]
    struct Input {
        a: Vec<String>,
    }

    let mut vm = build_solidity(
        r#"
        contract Testing {
            function testStringVector(bytes memory buffer) public pure returns (string[] memory) {
                string[] memory vec = abi.borshDecode(buffer, (string[]));
                return vec;
            }
        }
        "#,
    );

    vm.constructor("Testing", &[]);
    let data = Input {
        a: vec![
            "coffee".to_string(),
            "tea".to_string(),
            "cappuccino".to_string(),
        ],
    };
    let encoded = data.try_to_vec().unwrap();
    let returns = vm.function("testStringVector", &[Token::Bytes(encoded)], &[], None);
    let vec = returns[0].clone().into_array().unwrap();
    assert_eq!(vec.len(), 3);
    assert_eq!(vec[0].clone().into_string().unwrap(), "coffee");
    assert_eq!(vec[1].clone().into_string().unwrap(), "tea");
    assert_eq!(vec[2].clone().into_string().unwrap(), "cappuccino");
}

#[test]
fn struct_within_struct() {
    #[derive(Debug, BorshSerialize)]
    struct NoPadStruct {
        a: u32,
        b: u32,
    }

    #[derive(Debug, BorshSerialize)]
    struct PaddedStruct {
        a: u128,
        b: u8,
        c: [u8; 32],
    }

    #[derive(Debug, BorshSerialize)]
    struct NonConstantStruct {
        a: u64,
        b: Vec<String>,
        no_pad: NoPadStruct,
        pad: PaddedStruct,
    }

    let mut vm = build_solidity(
        r#"
    contract Testing {
        struct noPadStruct {
            uint32 a;
            uint32 b;
        }

        struct PaddedStruct {
            uint128 a;
            uint8 b;
            bytes32 c;
        }

        struct NonConstantStruct {
            uint64 a;
            string[] b;
            noPadStruct noPad;
            PaddedStruct pad;
        }

        function testStruct(bytes memory buffer) public pure {
            NonConstantStruct memory str = abi.borshDecode(buffer, (NonConstantStruct));
            assert(str.a == 890234);
            assert(str.b.length == 2);
            assert(str.b[0] == "tea");
            assert(str.b[1] == "coffee");
            assert(str.noPad.a == 89123);
            assert(str.noPad.b == 12354);
            assert(str.pad.a == 988834);
            assert(str.pad.b == 129);
            assert(str.pad.c == "tea_is_good");
        }
    }
        "#,
    );

    vm.constructor("Testing", &[]);
    let no_pad = NoPadStruct { a: 89123, b: 12354 };
    let mut tea_is_good = b"tea_is_good".to_vec();
    tea_is_good.append(&mut vec![0; 21]);
    let pad = PaddedStruct {
        a: 988834,
        b: 129,
        c: <[u8; 32]>::try_from(tea_is_good).unwrap(),
    };
    let data = NonConstantStruct {
        a: 890234,
        b: vec!["tea".to_string(), "coffee".to_string()],
        no_pad,
        pad,
    };
    let encoded = data.try_to_vec().unwrap();
    let _ = vm.function("testStruct", &[Token::Bytes(encoded)], &[], None);
}

#[test]
fn struct_in_array() {
    #[derive(Debug, BorshSerialize)]
    struct NoPadStruct {
        a: u32,
        b: u32,
    }

    #[derive(Debug, BorshSerialize)]
    struct PaddedStruct {
        a: u128,
        b: u8,
        c: [u8; 32],
    }

    #[derive(Debug, BorshSerialize)]
    struct Input1 {
        item_1: NoPadStruct,
        item_2: PaddedStruct,
    }

    #[derive(Debug, BorshSerialize)]
    struct Input2 {
        item_1: [i32; 4],
        item_2: [NoPadStruct; 2],
        item_3: Vec<NoPadStruct>,
    }

    #[derive(Debug, BorshSerialize)]
    struct Input3 {
        vec: Vec<NoPadStruct>,
    }

    let mut vm = build_solidity(
        r#"
    contract Testing {
        struct NoPadStruct {
            uint32 a;
            uint32 b;
        }

        struct PaddedStruct {
            uint128 a;
            uint8 b;
            bytes32 c;
        }

        function twoStructs(bytes memory buffer) public pure {
            (NoPadStruct memory a, PaddedStruct memory b) = abi.borshDecode(buffer, (NoPadStruct, PaddedStruct));
            assert(a.a == 945);
            assert(a.b == 7453);
            assert(b.a == 1);
            assert(b.b == 3);
            assert(b.c == "there_is_padding_here");
        }

        function fixedArrays(bytes memory buffer) public pure {
            (int32[4] memory a, NoPadStruct[2] memory b, NoPadStruct[] memory c) =
            abi.borshDecode(buffer, (int32[4], NoPadStruct[2], NoPadStruct[]));

            assert(a[0] == 1);
            assert(a[1] == -298);
            assert(a[2] == 3);
            assert(a[3] == -434);

            assert(b[0].a == 1);
            assert(b[0].b == 2);
            assert(b[1].a == 3);
            assert(b[1].b == 4);

            assert(c.length == 3);
            assert(c[0].a == 1623);
            assert(c[0].b == 43279);
            assert(c[1].a == 41234);
            assert(c[1].b == 98375);
            assert(c[2].a == 945);
            assert(c[2].b == 7453);
        }

        function primitiveDynamic(bytes memory buffer) public pure {
            NoPadStruct[] memory vec = abi.borshDecode(buffer, (NoPadStruct[]));

            assert(vec.length == 2);
            assert(vec[0].a == 5);
            assert(vec[0].b == 6);
            assert(vec[1].a == 7);
            assert(vec[1].b == 8);
        }

    }
        "#,
    );

    vm.constructor("Testing", &[]);
    let mut bytes_string = b"there_is_padding_here".to_vec();
    bytes_string.append(&mut vec![0; 11]);
    let input = Input1 {
        item_1: NoPadStruct { a: 945, b: 7453 },
        item_2: PaddedStruct {
            a: 1,
            b: 3,
            c: <[u8; 32]>::try_from(bytes_string).unwrap(),
        },
    };
    let encoded = input.try_to_vec().unwrap();
    let _ = vm.function("twoStructs", &[Token::Bytes(encoded)], &[], None);

    let input = Input2 {
        item_1: [1, -298, 3, -434],
        item_2: [NoPadStruct { a: 1, b: 2 }, NoPadStruct { a: 3, b: 4 }],
        item_3: vec![
            NoPadStruct { a: 1623, b: 43279 },
            NoPadStruct { a: 41234, b: 98375 },
            NoPadStruct { a: 945, b: 7453 },
        ],
    };
    let encoded = input.try_to_vec().unwrap();
    let _ = vm.function("fixedArrays", &[Token::Bytes(encoded)], &[], None);

    let input = Input3 {
        vec: vec![NoPadStruct { a: 5, b: 6 }, NoPadStruct { a: 7, b: 8 }],
    };
    let encoded = input.try_to_vec().unwrap();
    let _ = vm.function("primitiveDynamic", &[Token::Bytes(encoded)], &[], None);
}

#[test]
fn arrays() {
    #[derive(Debug, BorshSerialize, Default, Clone)]
    struct NonConstantStruct {
        a: u64,
        b: Vec<String>,
    }

    #[derive(Debug, BorshSerialize)]
    struct Input1 {
        complex_array: Vec<NonConstantStruct>,
    }

    #[derive(Debug, BorshSerialize)]
    struct Input2 {
        vec: Vec<i16>,
    }

    #[derive(Debug, BorshSerialize)]
    struct Input3 {
        multi_dim: [[i8; 2]; 3],
    }

    let mut vm = build_solidity(
        r#"
    contract Testing {
        struct NonConstantStruct {
            uint64 a;
            string[] b;
        }

        function decodeComplex(bytes memory buffer) public view {
            NonConstantStruct[] memory vec = abi.borshDecode(buffer, (NonConstantStruct[]));

            assert(vec.length == 2);

            assert(vec[0].a == 897);
            assert(vec[0].b[0] == "tea");
            assert(vec[0].b[1] == "coffee");

            assert(vec[1].a == 74123);
            assert(vec[1].b[0] == "cortado");
            assert(vec[1].b[1] == "cappuccino");
        }

        function dynamicArray(bytes memory buffer) public view {
            int16[] memory vec = abi.borshDecode(buffer, (int16[]));

            assert(vec.length == 3);

            assert(vec[0] == -90);
            assert(vec[1] == 5523);
            assert(vec[2] == -89);
        }

        function decodeMultiDim(bytes memory buffer) public view {
            int8[2][3] memory vec = abi.borshDecode(buffer, (int8[2][3]));

            print("{}".format(vec[0][1]));
            assert(vec[0][0] == 1);
            assert(vec[0][1] == 2);
            assert(vec[1][0] == 4);
            assert(vec[1][1] == 5);
            assert(vec[2][0] == 6);
            assert(vec[2][1] == 7);
        }
    }
        "#,
    );

    vm.constructor("Testing", &[]);
    let input = Input1 {
        complex_array: vec![
            NonConstantStruct {
                a: 897,
                b: vec!["tea".to_string(), "coffee".to_string()],
            },
            NonConstantStruct {
                a: 74123,
                b: vec!["cortado".to_string(), "cappuccino".to_string()],
            },
        ],
    };
    let encoded = input.try_to_vec().unwrap();
    let _ = vm.function("decodeComplex", &[Token::Bytes(encoded)], &[], None);

    let input = Input2 {
        vec: vec![-90, 5523, -89],
    };
    let encoded = input.try_to_vec().unwrap();
    let _ = vm.function("dynamicArray", &[Token::Bytes(encoded)], &[], None);

    let input = Input3 {
        multi_dim: [[1, 2], [4, 5], [6, 7]],
    };
    let encoded = input.try_to_vec().unwrap();
    let _ = vm.function("decodeMultiDim", &[Token::Bytes(encoded)], &[], None);
}

#[test]
fn multi_dimensional_arrays() {
    #[derive(Debug, BorshSerialize)]
    struct PaddedStruct {
        a: u128,
        b: u8,
        c: [u8; 32],
    }

    #[derive(Debug, BorshSerialize)]
    struct Input1 {
        item_1: Vec<[[PaddedStruct; 2]; 3]>,
        item_2: i16,
    }

    #[derive(Debug, BorshSerialize)]
    struct Input2 {
        vec: Vec<[[u16; 4]; 2]>,
    }

    #[derive(Debug, BorshSerialize)]
    struct Input3 {
        vec: Vec<u16>,
    }

    let mut vm = build_solidity(
        r#"
    contract Testing {
        struct PaddedStruct {
            uint128 a;
            uint8 b;
            bytes32 c;
        }

        function multiDimStruct(bytes memory buffer) public pure {
            (PaddedStruct[2][3][] memory vec, int16 g) = abi.borshDecode(buffer, (PaddedStruct[2][3][], int16));

            assert(vec.length == 1);

            assert(vec[0][0][0].a == 56);
            assert(vec[0][0][0].b == 1);
            assert(vec[0][0][0].c == "oi");

            assert(vec[0][0][1].a == 78);
            assert(vec[0][0][1].b == 6);
            assert(vec[0][0][1].c == "bc");

            assert(vec[0][1][0].a == 89);
            assert(vec[0][1][0].b == 4);
            assert(vec[0][1][0].c == "sn");

            assert(vec[0][1][1].a == 42);
            assert(vec[0][1][1].b == 56);
            assert(vec[0][1][1].c == "cn");

            assert(vec[0][2][0].a == 23);
            assert(vec[0][2][0].b == 78);
            assert(vec[0][2][0].c == "fr");

            assert(vec[0][2][1].a == 445);
            assert(vec[0][2][1].b == 46);
            assert(vec[0][2][1].c == "br");

            assert(g == -90);
        }

        function multiDimInt(bytes memory buffer) public pure {
            uint16[4][2][] memory vec = abi.borshDecode(buffer, (uint16[4][2][]));

            assert(vec.length == 2);

            assert(vec[0][0][0] == 1);
            assert(vec[0][0][1] == 2);
            assert(vec[0][0][2] == 3);
            assert(vec[0][0][3] == 4);

            assert(vec[0][1][0] == 5);
            assert(vec[0][1][1] == 6);
            assert(vec[0][1][2] == 7);
            assert(vec[0][1][3] == 8);

            assert(vec[1][0][0] == 9);
            assert(vec[1][0][1] == 10);
            assert(vec[1][0][2] == 11);
            assert(vec[1][0][3] == 12);

            assert(vec[1][1][0] == 13);
            assert(vec[1][1][1] == 14);
            assert(vec[1][1][2] == 15);
            assert(vec[1][1][3] == 16);
        }

        function uniqueDim(bytes memory buffer) public pure {
            uint16[] memory vec = abi.borshDecode(buffer, (uint16[]));

            assert(vec.length == 5);

            assert(vec[0] == 9);
            assert(vec[1] == 3);
            assert(vec[2] == 4);
            assert(vec[3] == 90);
            assert(vec[4] == 834);
        }
    }
        "#,
    );
    vm.constructor("Testing", &[]);
    let mut response: Vec<u8> = vec![0; 32];

    let input = Input1 {
        item_1: vec![[
            [
                PaddedStruct {
                    a: 56,
                    b: 1,
                    c: create_response(&mut response, b"oi"),
                },
                PaddedStruct {
                    a: 78,
                    b: 6,
                    c: create_response(&mut response, b"bc"),
                },
            ],
            [
                PaddedStruct {
                    a: 89,
                    b: 4,
                    c: create_response(&mut response, b"sn"),
                },
                PaddedStruct {
                    a: 42,
                    b: 56,
                    c: create_response(&mut response, b"cn"),
                },
            ],
            [
                PaddedStruct {
                    a: 23,
                    b: 78,
                    c: create_response(&mut response, b"fr"),
                },
                PaddedStruct {
                    a: 445,
                    b: 46,
                    c: create_response(&mut response, b"br"),
                },
            ],
        ]],
        item_2: -90,
    };
    let encoded = input.try_to_vec().unwrap();
    let _ = vm.function("multiDimStruct", &[Token::Bytes(encoded)], &[], None);

    let input = Input2 {
        vec: vec![
            [[1, 2, 3, 4], [5, 6, 7, 8]],
            [[9, 10, 11, 12], [13, 14, 15, 16]],
        ],
    };
    let encoded = input.try_to_vec().unwrap();
    let _ = vm.function("multiDimInt", &[Token::Bytes(encoded)], &[], None);

    let input = Input3 {
        vec: vec![9, 3, 4, 90, 834],
    };
    let encoded = input.try_to_vec().unwrap();
    let _ = vm.function("uniqueDim", &[Token::Bytes(encoded)], &[], None);
}

#[test]
fn empty_arrays() {
    #[derive(Debug, BorshSerialize)]
    struct S {
        f1: i64,
        f2: String,
    }

    #[derive(Debug, BorshSerialize)]
    struct Input {
        vec_1: Vec<S>,
        vec_2: Vec<String>,
    }

    let mut vm = build_solidity(
        r#"
    contract Testing {
        struct S {
            int64 f1;
            string f2;
        }

        function testEmpty(bytes memory buffer) public pure {
            (S[] memory vec_1, string[] memory vec_2) = abi.borshDecode(buffer, (S[], string[]));

            assert(vec_1.length == 0);
            assert(vec_2.length == 0);
        }
    }
        "#,
    );
    vm.constructor("Testing", &[]);

    let input = Input {
        vec_1: vec![],
        vec_2: vec![],
    };
    let encoded = input.try_to_vec().unwrap();
    let _ = vm.function("testEmpty", &[Token::Bytes(encoded)], &[], None);
}

#[test]
fn external_function() {
    #[derive(Debug, BorshSerialize)]
    struct Input {
        selector: [u8; 4],
        address: [u8; 32],
    }

    let mut vm = build_solidity(
        r#"
    contract Testing {
        function testExternalFunction(bytes memory buffer) public view returns (bytes4, address) {
            function (uint8) external returns (int8) fPtr = abi.borshDecode(buffer, (function (uint8) external returns (int8)));
            return (fPtr.selector, fPtr.address);
        }
    }
        "#,
    );

    vm.constructor("Testing", &[]);
    let input = Input {
        selector: [1, 2, 3, 4],
        address: [
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23,
            24, 25, 26, 27, 28, 29, 30, 31,
        ],
    };
    let encoded = input.try_to_vec().unwrap();
    let returns = vm.function("testExternalFunction", &[Token::Bytes(encoded)], &[], None);

    let selector = returns[0].clone().into_fixed_bytes().unwrap();
    assert_eq!(selector, input.selector);

    let address = returns[1].clone().into_fixed_bytes().unwrap();
    assert_eq!(address, input.address);
}

#[test]
fn bytes_arrays() {
    #[derive(Debug, BorshSerialize)]
    struct Input {
        item_1: [[u8; 4]; 2],
        item_2: Vec<[u8; 5]>,
    }

    let mut vm = build_solidity(
        r#"
        contract Testing {
            function testByteArrays(bytes memory buffer) public view {
                (bytes4[2] memory arr, bytes5[] memory vec) = abi.borshDecode(buffer, (bytes4[2], bytes5[]));

                assert(arr[0] == "abcd");
                assert(arr[1] == "efgh");

                assert(vec.length == 2);
                assert(vec[0] == "12345");
                assert(vec[1] == "67890");
            }
        }
        "#,
    );

    vm.constructor("Testing", &[]);
    let input = Input {
        item_1: [b"abcd".to_owned(), b"efgh".to_owned()],
        item_2: vec![b"12345".to_owned(), b"67890".to_owned()],
    };
    let encoded = input.try_to_vec().unwrap();
    let _ = vm.function("testByteArrays", &[Token::Bytes(encoded)], &[], None);
}

#[test]
#[should_panic(expected = "unexpected return 0x100000000")]
fn different_types() {
    #[derive(Debug, BorshSerialize)]
    struct Input1 {
        a: i32,
        b: u64,
    }

    let mut vm = build_solidity(
        r#"
    contract Testing {
        function testByteArrays(bytes memory buffer) public view {
            (bytes4[2] memory arr, bytes5[] memory vec) = abi.borshDecode(buffer, (bytes4[2], bytes5[]));

            assert(arr[0] == "abcd");
            assert(arr[1] == "efgh");

            assert(vec.length == 2);
            assert(vec[0] == "12345");
            assert(vec[1] == "67890");
        }
    }
        "#,
    );

    vm.constructor("Testing", &[]);
    let input = Input1 { a: -789, b: 14234 };
    let encoded = input.try_to_vec().unwrap();
    let _ = vm.function("testByteArrays", &[Token::Bytes(encoded)], &[], None);
}

#[test]
#[should_panic(expected = "unexpected return 0x100000000")]
fn more_elements() {
    #[derive(Debug, BorshSerialize)]
    struct Input {
        vec: [i64; 4],
    }

    let mut vm = build_solidity(
        r#"
        contract Testing {
            function wrongNumber(bytes memory buffer) public view {
               int64[5] memory vec = abi.borshDecode(buffer, (int64[5]));

               assert(vec[1] == 0);
            }
        }
        "#,
    );

    vm.constructor("Testing", &[]);

    let input = Input { vec: [1, 4, 5, 6] };
    let encoded = input.try_to_vec().unwrap();
    let _ = vm.function("wrongNumber", &[Token::Bytes(encoded)], &[], None);
}

#[test]
#[should_panic(expected = "unexpected return 0x100000000")]
fn extra_element() {
    #[derive(Debug, BorshSerialize)]
    struct Input {
        vec: Vec<i64>,
    }

    let mut vm = build_solidity(
        r#"
        contract Testing {
            function extraElement(bytes memory buffer) public pure {
               (int64[] memory vec, int32 g) = abi.borshDecode(buffer, (int64[], int32));

               assert(vec[1] == 0);
               assert(g == 3);
            }
        }
        "#,
    );

    vm.constructor("Testing", &[]);
    let input = Input {
        vec: vec![-90, 89, -2341],
    };

    let encoded = input.try_to_vec().unwrap();
    let _ = vm.function("extraElement", &[Token::Bytes(encoded)], &[], None);
}

#[test]
#[should_panic(expected = "unexpected return 0x100000000")]
fn invalid_type() {
    #[derive(Debug, BorshSerialize)]
    struct Input {
        item: u64,
    }

    let mut vm = build_solidity(
        r#"
    contract Testing {
        function invalidType(bytes memory buffer) public pure {
           int64[] memory vec = abi.borshDecode(buffer, (int64[]));

           assert(vec[1] == 0);
        }
    }
    "#,
    );

    vm.constructor("Testing", &[]);

    let input = Input { item: 5 };
    let encoded = input.try_to_vec().unwrap();
    let _ = vm.function("invalidType", &[Token::Bytes(encoded)], &[], None);
}

#[test]
#[should_panic(expected = "unexpected return 0x100000000")]
fn longer_buffer() {
    #[derive(Debug, BorshSerialize)]
    struct Input {
        item_1: u64,
        item_2: u64,
    }

    let mut vm = build_solidity(
        r#"
    contract Testing {
        function testLongerBuffer(bytes memory buffer) public view {
            uint64 a = abi.borshDecode(buffer, (uint64));

            assert(a == 4);
        }
    }
        "#,
    );

    vm.constructor("Testing", &[]);

    let input = Input {
        item_1: 4,
        item_2: 5,
    };
    let encoded = input.try_to_vec().unwrap();
    let _ = vm.function("testLongerBuffer", &[Token::Bytes(encoded)], &[], None);
}

#[test]
#[should_panic(expected = "unexpected return 0x100000000")]
fn longer_buffer_array() {
    #[derive(Debug, BorshSerialize)]
    struct Input {
        item_1: u64,
        item_2: [u32; 4],
    }

    let mut vm = build_solidity(
        r#"
        contract Testing {
            function testLongerBuffer(bytes memory buffer) public view {
                (uint64 a, uint32[3] memory b) = abi.borshDecode(buffer, (uint64, uint32[3]));

                assert(a == 4);
                assert(b[0] == 1);
                assert(b[1] == 2);
                assert(b[2] == 3);
            }
        }        "#,
    );
    vm.constructor("Testing", &[]);

    let input = Input {
        item_1: 23434,
        item_2: [1, 2, 3, 4],
    };
    let encoded = input.try_to_vec().unwrap();
    let _ = vm.function("testLongerBuffer", &[Token::Bytes(encoded)], &[], None);
}

#[test]
fn dynamic_array_of_array() {
    #[derive(Debug, BorshSerialize)]
    struct Input {
        vec: Vec<[i32; 2]>,
    }

    let mut vm = build_solidity(
        r#"
        contract Testing {
            function testArrayAssign(bytes memory buffer) public pure {
                int32[2][] memory vec = abi.borshDecode(buffer, (int32[2][]));

                assert(vec.length == 2);

                assert(vec[0][0] == 0);
                assert(vec[0][1] == 1);
                assert(vec[1][0] == 2);
                assert(vec[1][1] == -3);
            }
        }
        "#,
    );

    vm.constructor("Testing", &[]);
    let input = Input {
        vec: vec![[0, 1], [2, -3]],
    };
    let encoded = input.try_to_vec().unwrap();
    let _ = vm.function("testArrayAssign", &[Token::Bytes(encoded)], &[], None);
}

#[test]
fn test_struct_validation() {
    #[derive(Debug, BorshSerialize)]
    struct MyStruct {
        b: [u8; 32],
        c: i8,
        d: String,
    }

    #[derive(Debug, BorshSerialize)]
    struct Input {
        b: u128,
        m_str: MyStruct,
    }

    let mut vm = build_solidity(
        r#"
    contract Testing {
        struct myStruct {
            bytes32 b;
            int8 c;
            string d;
        }


        function test(bytes memory buffer) public pure {
            (uint128 b, myStruct memory m_str) = abi.borshDecode(buffer, (uint128, myStruct));

            assert(m_str.b == "struct");
            assert(m_str.c == 1);
            assert(m_str.d == "string");
            assert(b == 3);
        }
    }
        "#,
    );

    vm.constructor("Testing", &[]);
    let mut bytes_string = b"struct".to_vec();
    bytes_string.append(&mut vec![0; 26]);

    let input = Input {
        b: 3,
        m_str: MyStruct {
            b: <[u8; 32]>::try_from(bytes_string).unwrap(),
            c: 1,
            d: "string".to_string(),
        },
    };
    let encoded = input.try_to_vec().unwrap();
    let _ = vm.function("test", &[Token::Bytes(encoded)], &[], None);
}

#[test]
#[should_panic(expected = "unexpected return 0x100000000")]
fn test_struct_validation_invalid() {
    #[derive(Debug, BorshSerialize)]
    struct MyStruct {
        b: [u8; 32],
        c: i8,
        d: String,
    }

    #[derive(Debug, BorshSerialize)]
    struct Input {
        m_str: MyStruct,
    }

    let mut vm = build_solidity(
        r#"
    contract Testing {
        struct myStruct {
            bytes32 b;
            int8 c;
            string d;
        }


        function test(bytes memory buffer) public pure {
            (uint128 b, myStruct memory m_str) = abi.borshDecode(buffer, (uint128, myStruct));

            assert(m_str.b == "struct");
            assert(m_str.c == 1);
            assert(m_str.d == "string");
            assert(b == 3);
        }
    }
        "#,
    );

    vm.constructor("Testing", &[]);
    let mut bytes_string = b"struct".to_vec();
    bytes_string.append(&mut vec![0; 26]);

    let input = Input {
        m_str: MyStruct {
            b: <[u8; 32]>::try_from(bytes_string).unwrap(),
            c: 1,
            d: "string".to_string(),
        },
    };
    let encoded = input.try_to_vec().unwrap();
    let _ = vm.function("test", &[Token::Bytes(encoded)], &[], None);
}
