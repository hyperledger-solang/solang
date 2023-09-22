// SPDX-License-Identifier: Apache-2.0

use crate::borsh_encoding::BorshToken;
use crate::build_solidity;
use num_bigint::BigInt;

#[test]
fn user_defined_oper() {
    let mut runtime = build_solidity(
        r#"
        type Bitmap is int256;

        function eq(Bitmap a, Bitmap b) pure returns (bool) {
            return Bitmap.unwrap(a) == Bitmap.unwrap(b);
        }

        function ne(Bitmap a, Bitmap b) pure returns (bool) {
            return Bitmap.unwrap(a) != Bitmap.unwrap(b);
        }

        function gt(Bitmap a, Bitmap b) pure returns (bool) {
            return Bitmap.unwrap(a) > Bitmap.unwrap(b);
        }

        function gte(Bitmap a, Bitmap b) pure returns (bool) {
            return Bitmap.unwrap(a) >= Bitmap.unwrap(b);
        }

        function lt(Bitmap a, Bitmap b) pure returns (bool) {
            return Bitmap.unwrap(a) < Bitmap.unwrap(b);
        }

        function lte(Bitmap a, Bitmap b) pure returns (bool) {
            return Bitmap.unwrap(a) <= Bitmap.unwrap(b);
        }

        using {eq as ==, ne as !=, lt as <, lte as <=, gt as >, gte as >=} for Bitmap global;

        // arithmetic
        function neg(Bitmap a) pure returns (Bitmap) {
            return Bitmap.wrap(-Bitmap.unwrap(a));
        }

        function sub(Bitmap a, Bitmap b) pure returns (Bitmap) {
            return Bitmap.wrap(Bitmap.unwrap(a) - Bitmap.unwrap(b));
        }

        function add(Bitmap a, Bitmap b) pure returns (Bitmap) {
            return Bitmap.wrap(Bitmap.unwrap(a) + Bitmap.unwrap(b));
        }

        function mul(Bitmap a, Bitmap b) pure returns (Bitmap) {
            return Bitmap.wrap(Bitmap.unwrap(a) * Bitmap.unwrap(b));
        }

        function div(Bitmap a, Bitmap b) pure returns (Bitmap) {
            return Bitmap.wrap(Bitmap.unwrap(a) / Bitmap.unwrap(b));
        }

        function mod(Bitmap a, Bitmap b) pure returns (Bitmap) {
            return Bitmap.wrap(Bitmap.unwrap(a) % Bitmap.unwrap(b));
        }

        using {neg as -, sub as -, add as +, mul as *, div as /, mod as %} for Bitmap global;

        function and(Bitmap a, Bitmap b) pure returns (Bitmap) {
            return Bitmap.wrap(Bitmap.unwrap(a) & Bitmap.unwrap(b));
        }

        function or(Bitmap a, Bitmap b) pure returns (Bitmap) {
            return Bitmap.wrap(Bitmap.unwrap(a) | Bitmap.unwrap(b));
        }

        function xor(Bitmap a, Bitmap b) pure returns (Bitmap) {
            return Bitmap.wrap(Bitmap.unwrap(a) ^ Bitmap.unwrap(b));
        }

        function cpl(Bitmap a) pure returns (Bitmap) {
            return Bitmap.wrap(~Bitmap.unwrap(a));
        }

        using {and as &, or as |, xor as ^, cpl as ~} for Bitmap global;

        contract C {
            Bitmap a;

            function test_cmp() public view {
                Bitmap zero = Bitmap.wrap(0);
                Bitmap one = Bitmap.wrap(1);
                Bitmap one2 = Bitmap.wrap(1);

                assert(zero != one);
                assert(zero < one);
                assert(zero <= one);
                assert(one == one2);
                assert(one <= one2);
                assert(one >= zero);
                assert(one >= one2);
                assert(one > zero);
            }

            function test_arith() public view {
                Bitmap two = Bitmap.wrap(2);
                Bitmap three = Bitmap.wrap(3);
                Bitmap seven = Bitmap.wrap(7);

                assert(Bitmap.unwrap(two + three) == 5);
                assert(Bitmap.unwrap(two - three) == -1);
                assert(Bitmap.unwrap(two * three) == 6);
                assert(Bitmap.unwrap(seven / two) == 3);
                assert(Bitmap.unwrap(seven / two) == 3);
                assert(Bitmap.unwrap(-seven) == -7);
            }

            function test_bit() public view {
                Bitmap two = Bitmap.wrap(2);
                Bitmap three = Bitmap.wrap(3);
                Bitmap seven = Bitmap.wrap(7);
                Bitmap eight = Bitmap.wrap(8);

                assert(Bitmap.unwrap(two | three) == 3);
                assert(Bitmap.unwrap(eight | three) == 11);
                assert(Bitmap.unwrap(eight & three) == 0);
                assert(Bitmap.unwrap(eight & seven) == 0);
                assert(Bitmap.unwrap(two ^ three) == 1);
                assert((Bitmap.unwrap(~three) & 255) == 252);
            }
        }"#,
    );

    let data_account = runtime.initialize_data_account();
    runtime
        .function("new")
        .accounts(vec![("dataAccount", data_account)])
        .call();

    runtime.function("test_cmp").call();
    runtime.function("test_arith").call();
    runtime.function("test_bit").call();
}

#[test]
fn using_for_struct() {
    let mut vm = build_solidity(
        r#"
struct Pet {
    string name;
    uint8 age;
}

library Info {
    function isCat(Pet memory myPet) public pure returns (bool) {
        return myPet.name == "cat";
    }

    function setAge(Pet memory myPet, uint8 age) pure public {
        myPet.age = age;
    }
}

contract C {
    using Info for Pet;

    function testPet(string memory name, uint8 age) pure public returns (bool) {
        Pet memory my_pet = Pet(name, age);
        return my_pet.isCat();
    }

    function changeAge(Pet memory myPet) public pure returns (Pet memory) {
        myPet.setAge(5);
        return myPet;
    }

}
        "#,
    );

    let data_account = vm.initialize_data_account();

    vm.function("new")
        .accounts(vec![("dataAccount", data_account)])
        .call();

    let res = vm
        .function("testPet")
        .arguments(&[
            BorshToken::String("cat".to_string()),
            BorshToken::Uint {
                width: 8,
                value: BigInt::from(2u8),
            },
        ])
        .call()
        .unwrap();

    assert_eq!(res, BorshToken::Bool(true));

    let res = vm
        .function("changeAge")
        .arguments(&[BorshToken::Tuple(vec![
            BorshToken::String("cat".to_string()),
            BorshToken::Uint {
                width: 8,
                value: BigInt::from(2u8),
            },
        ])])
        .call()
        .unwrap();

    assert_eq!(
        res,
        BorshToken::Tuple(vec![
            BorshToken::String("cat".to_string()),
            BorshToken::Uint {
                width: 8,
                value: BigInt::from(5u8),
            }
        ])
    );
}

#[test]
fn using_overload() {
    let mut vm = build_solidity(
        r#"
        library MyBytes {
    function push(bytes memory b, uint8[] memory a) pure public returns (bool) {
        return b[0] == bytes1(a[0]) && b[1] == bytes1(a[1]);
    }
}

contract C {
    using MyBytes for bytes;

    function check() public pure returns (bool) {
        bytes memory b;
        b.push(1);
        b.push(2);
        uint8[] memory vec = new uint8[](2);
        vec[0] = 1;
        vec[1] = 2;
        return b.push(vec);
    }
}

        "#,
    );

    let data_account = vm.initialize_data_account();
    vm.function("new")
        .accounts(vec![("dataAccount", data_account)])
        .call();

    let res = vm.function("check").call().unwrap();

    assert_eq!(res, BorshToken::Bool(true));
}

#[test]
fn using_function_for_struct() {
    let mut vm = build_solidity(
        r#"
struct Pet {
    string name;
    uint8 age;
}

library Info {
    function isCat(Pet memory myPet) public pure returns (bool) {
        return myPet.name == "cat";
    }

    function setAge(Pet memory myPet, uint8 age) pure public {
        myPet.age = age;
    }
}

contract C {
    using {Info.isCat, Info.setAge} for Pet;

    function testPet(string memory name, uint8 age) pure public returns (bool) {
        Pet memory my_pet = Pet(name, age);
        return my_pet.isCat();
    }

    function changeAge(Pet memory myPet) public pure returns (Pet memory) {
        myPet.setAge(5);
        return myPet;
    }

}
        "#,
    );

    let data_account = vm.initialize_data_account();

    vm.function("new")
        .accounts(vec![("dataAccount", data_account)])
        .call();

    let res = vm
        .function("testPet")
        .arguments(&[
            BorshToken::String("cat".to_string()),
            BorshToken::Uint {
                width: 8,
                value: BigInt::from(2u8),
            },
        ])
        .call()
        .unwrap();

    assert_eq!(res, BorshToken::Bool(true));

    let res = vm
        .function("changeAge")
        .arguments(&[BorshToken::Tuple(vec![
            BorshToken::String("cat".to_string()),
            BorshToken::Uint {
                width: 8,
                value: BigInt::from(2u8),
            },
        ])])
        .call()
        .unwrap();

    assert_eq!(
        res,
        BorshToken::Tuple(vec![
            BorshToken::String("cat".to_string()),
            BorshToken::Uint {
                width: 8,
                value: BigInt::from(5u8),
            }
        ])
    );
}

#[test]
fn using_function_overload() {
    let mut vm = build_solidity(
        r#"
        library LibInLib {
            function get0(bytes x) public pure returns (bytes1) {
                return x[0];
            }

            function get1(bytes x) public pure returns (bytes1) {
                return x[1];
            }
        }

        library MyBytes {
            using {LibInLib.get0, LibInLib.get1} for bytes;

            function push(bytes memory b, uint8[] memory a) pure public returns (bool) {
                return b.get0() == a[0] && b.get1()== a[1];
            }
        }

        contract C {
            using {MyBytes.push} for bytes;

            function check() public pure returns (bool) {
                bytes memory b;
                b.push(1);
                b.push(2);
                uint8[] memory vec = new uint8[](2);
                vec[0] = 1;
                vec[1] = 2;
                return b.push(vec);
            }
        }"#,
    );

    let data_account = vm.initialize_data_account();
    vm.function("new")
        .accounts(vec![("dataAccount", data_account)])
        .call();

    let res = vm.function("check").call().unwrap();

    assert_eq!(res, BorshToken::Bool(true));
}
