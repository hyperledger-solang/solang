use parity_scale_codec::{Decode, Encode};
use parity_scale_codec_derive::{Decode, Encode};

use crate::{build_solidity, first_error, no_errors, parse_and_resolve};
use num_bigint::BigInt;
use num_bigint::Sign;
use rand::Rng;
use solang::Target;

#[test]
fn celcius_and_fahrenheit() {
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct Val(u32);

    // parse
    let mut runtime = build_solidity(
        "
        contract test {
            function celcius2fahrenheit(int32 celcius) pure public returns (int32) {
                int32 fahrenheit = celcius * 9 / 5 + 32;

                return fahrenheit;
            }

            function fahrenheit2celcius(uint32 fahrenheit) pure public returns (uint32) {
                return (fahrenheit - 32) * 5 / 9;
            }
        }",
    );

    runtime.function("celcius2fahrenheit", Val(10).encode());

    assert_eq!(runtime.vm.output, Val(50).encode());

    runtime.function("fahrenheit2celcius", Val(50).encode());

    assert_eq!(runtime.vm.output, Val(10).encode());
}

#[test]
fn digits() {
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct Val32(u32);
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct Val64(u64);

    // parse
    let mut runtime = build_solidity(
        "
        contract test {
            function digitslen(uint64 val) pure public returns (uint32) {
                uint32 count = 0;
                while (val > 0) {
                    count++;
                    val /= 10;
                }
                if (count == 0) {
                    count = 1;
                }
                return count;
            }

            function sumdigits(int64 val) pure public returns (uint32) {
                uint32 sum = 0;

                while (val > 0) {
                    sum += uint32(val % 10);
                    val= val / 10;
                }

                return sum;
            }
        }",
    );

    runtime.function("digitslen", Val64(1234567).encode());

    assert_eq!(runtime.vm.output, Val32(7).encode());

    runtime.function("sumdigits", Val64(123456789).encode());

    assert_eq!(runtime.vm.output, Val32(45).encode());
}

#[test]
fn large_loops() {
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct Val32(u32);
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct Val64(u64);

    // parse
    let mut runtime = build_solidity(
        "
        contract test {
            function foo(uint val) pure public returns (uint) {
                for (uint i =0 ; i < 100; i++ ) {
                    val += i + 10;
                }

                return val;
            }

            function baz(int val) pure public returns (int) {
                return val * 1000_000;
            }

            function bar() public {
                assert(foo(10) == 5960);
                assert(baz(7_000_123) == 7_000_123_000_000);
                assert(baz(7_000_123_456_678) == 7_000_123_456_678_000_000);
            }
        }",
    );

    runtime.function("bar", Vec::new());

    let mut args = Val64(7000).encode();
    args.resize(32, 0);

    runtime.function("baz", args);

    let mut rets = Val64(7000000000).encode();
    rets.resize(32, 0);

    assert_eq!(runtime.vm.output, rets);
}

#[test]
fn expressions() {
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct Val16(u16);
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct Val8(u8);

    // parse
    let mut runtime = build_solidity("
        contract test {
            // this is 2^254
            int constant large_value = 14474011154664524427946373126085988481658748083205070504932198000989141204992;

            function add_100(uint16 a) pure public returns (uint16) {
                a -= 200;
                a += 300;
                return a;
            }

            function clear_digit(uint8 a) pure public returns (uint8) {
                a /= 10;
                a *= 10;
                return a;
            }

            function low_digit(uint8 a) pure public returns (uint8) {
                a %= 10;
                return a;
            }

            function test_comparisons() pure public {
                {
                    // test comparisons work, if will work even if sign/unsigned is broken
                    uint64 left = 102;
                    uint64 right = 103;

                    assert(left < right);
                    assert(left <= right);
                    assert(left != right);
                    assert(right > left);
                    assert(right >= left);
                    assert(right == 103);
                    assert(left >= 102);
                    assert(right <= 103);
                    assert(!(right <= 102));
                }

                {
                    // check if unsigned compare works correctly (will fail if signed compare is done)
                    uint16 left = 102;
                    uint16 right = 0x8001;

                    assert(left < right);
                    assert(left <= right);
                    assert(left != right);
                    assert(right > left);
                    assert(right >= left);
                    assert(right == 0x8001);
                    assert(left >= 102);
                    assert(right <= 0x8001);
                    assert(!(right <= 102));
                }

                {
                    // check if signed compare works correctly (will fail if unsigned compare is done)
                    int left = -102;
                    int right = large_value;

                    assert(left < right);
                    assert(left <= right);
                    assert(left != right);
                    assert(right > left);
                    assert(right >= left);
                    assert(right == large_value);
                    assert(left >= -102);
                    assert(right <= large_value);
                    assert(!(right <= -102));
                }
            }

            function increments() public {
                uint a = 1;

                assert(a-- == 1);
                assert(a == 0);

                assert(a++ == 0);
                assert(a == 1);

                assert(--a == 0);
                assert(a == 0);

                assert(++a == 1);
                assert(a == 1);
            }
        }",
    );

    runtime.function("add_100", Val16(0xffc0).encode());

    assert_eq!(runtime.vm.output, Val16(36).encode());

    runtime.function("clear_digit", Val8(25).encode());

    assert_eq!(runtime.vm.output, Val8(20).encode());

    runtime.function("low_digit", Val8(25).encode());

    assert_eq!(runtime.vm.output, Val8(5).encode());

    runtime.function("test_comparisons", Vec::new());

    runtime.function("increments", Vec::new());
}

#[test]
fn test_cast_errors() {
    let ns = parse_and_resolve(
        "contract test {
            function foo(uint bar) public {
                bool is_nonzero = bar;
            }
        }",
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "conversion from uint256 to bool not possible"
    );

    let ns = parse_and_resolve(
        "contract test {
            function foobar(uint foo, int bar) public returns (bool) {
                return (foo < bar);
            }
        }",
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "implicit conversion would change sign from uint256 to int256"
    );

    let ns = parse_and_resolve(
        "contract test {
            function foobar(int32 foo, uint16 bar) public returns (bool) {
                foo = bar;
                return false;
            }
        }",
        Target::Substrate,
    );

    no_errors(ns.diagnostics);

    // int16 can be negative, so cannot be stored in uint32
    let ns = parse_and_resolve(
        "contract test {
            function foobar(uint32 foo, int16 bar) public returns (bool) {
                foo = bar;
                return false;
            }
        }",
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "implicit conversion would change sign from int16 to uint32"
    );

    let ns = parse_and_resolve(
        "contract foo {
            uint bar;

            function set_bar(uint32 b) public {
                bar = b;
            }

            function get_bar() public returns (uint32) {
                return uint32(bar);
            }
        }

        contract foo2 {
            enum X { Y1, Y2, Y3}
            X y;

            function set_x(uint32 b) public {
                y = X(b);
            }

            function get_x() public returns (uint32) {
                return uint32(y);
            }

            function set_enum_x(X b) public {
                set_x(uint32(b));
            }
        }",
        Target::Substrate,
    );

    no_errors(ns.diagnostics);
}

#[test]
#[should_panic]
fn divisions_by_zero() {
    // parse
    let mut runtime = build_solidity(
        "
        contract test {
            function do_test() public returns (uint){
                uint256 val = 100;

                return (val / 0);
            }
        }",
    );

    runtime.function("do_test", Vec::new());
}

#[test]
fn divisions() {
    // parse
    let mut runtime = build_solidity(
        "
        contract test {
            uint constant large = 101213131318098987934191741;
            function do_test() public returns (uint) {
                assert(large / 1 == large);
                assert(large / (large + 102) == 0);
                assert(large / large == 1);

                assert(large % 1 == 0);
                assert(large % (large + 102) == large);
                assert(large % large == 0);

                return 0;
            }
        }",
    );

    runtime.function("do_test", Vec::new());
}

#[test]
fn divisions64() {
    // parse
    let mut runtime = build_solidity(
        "
        contract test {
            uint64 constant large = 101213131318098987;
            function do_test() public returns (uint) {
                assert(large / 1 == large);
                assert(large / (large + 102) == 0);
                assert(large / large == 1);

                assert(large % 1 == 0);
                assert(large % (large + 102) == large);
                assert(large % large == 0);

                return 0;
            }
        }",
    );

    runtime.function("do_test", Vec::new());
}

#[test]
fn divisions128() {
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct Args(i128, i128);

    #[derive(Debug, PartialEq, Encode, Decode)]
    struct Rets(i128);

    // parse
    let mut runtime = build_solidity(
        "
        contract test {
            uint128 constant large = 101213131318098987;
            uint128 constant small = 99;

            int128 constant signed_large = 101213131318098987;
            int128 constant neg_signed_large = -101213131318098987;
            int128 constant signed_small = 99;

            function do_test() public returns (uint) {
                assert(large / 1 == large);
                assert(large / (large + 102) == 0);
                assert(large / large == 1);

                assert(large % 1 == 0);
                assert(large % (large + 102) == large);
                assert(large % large == 0);

                assert(small / 10 == 9);
                assert(small % 10 == 9);

                assert(large / 100000 == 1012131313180);
                assert(large % 100000 == 98987);

                return 0;
            }

            function do_signed_test() public returns (uint) {
                assert(signed_large / 1 == signed_large);
                assert(signed_large / (signed_large + 102) == 0);
                assert(signed_large / signed_large == 1);

                assert(signed_large % 1 == 0);
                assert(signed_large % (signed_large + 102) == signed_large);
                assert(signed_large % signed_large == 0);

                assert(signed_small / 10 == 9);
                assert(signed_small % 10 == 9);

                assert(signed_large / 100000 == 1012131313180);
                assert(signed_large % 100000 == 98987);

                assert(neg_signed_large / -100000 == 1012131313180);
                assert(signed_large / -100000 == -1012131313180);
                assert(-signed_large / 100000 == -1012131313180);

                assert(signed_large % -100000 == 98987);
                assert(-signed_large % 100000 == -98987);
                assert(-signed_large % -100000 == -98987);


                return 0;
            }

            function do_div(int128 x, int128 y) public returns (int128) {
                return x / y;
            }

            function return_neg() public returns (int128) {
                return -100;
            }

            function return_pos() public returns (int128) {
                return 255;
            }
        }",
    );

    runtime.function("do_test", Vec::new());

    runtime.function("return_neg", Vec::new());

    if let Ok(Rets(r)) = Rets::decode(&mut &runtime.vm.output[..]) {
        assert_eq!(r, -100);
    } else {
        panic!();
    }

    runtime.function("return_pos", Vec::new());

    if let Ok(Rets(r)) = Rets::decode(&mut &runtime.vm.output[..]) {
        assert_eq!(r, 255);
    } else {
        panic!();
    }

    runtime.function("do_div", Args(-9900, -100).encode());

    if let Ok(Rets(r)) = Rets::decode(&mut &runtime.vm.output[..]) {
        assert_eq!(r, 99);
    } else {
        panic!();
    }

    runtime.function("do_div", Args(-101213131318098987, -100000).encode());

    if let Ok(Rets(r)) = Rets::decode(&mut &runtime.vm.output[..]) {
        assert_eq!(r, 1012131313180);
    } else {
        panic!();
    }

    runtime.function("do_signed_test", Vec::new());
}

#[test]
fn divisions256() {
    // parse
    let mut runtime = build_solidity(
        "
        contract test {
            uint256 constant large = 101213131318098987;
            uint256 constant small = 99;
            function do_test() public returns (uint) {
                assert(large / 1 == large);
                assert(large / (large + 102) == 0);
                assert(large / large == 1);

                assert(large % 1 == 0);
                assert(large % (large + 102) == large);
                assert(large % large == 0);

                assert(small / 10 == 9);
                assert(small % 10 == 9);

                assert(large / 100000 == 1012131313180);
                assert(large % 100000 == 98987);

                return 0;
            }
        }",
    );

    runtime.function("do_test", Vec::new());
}

#[test]
fn complement() {
    // parse
    let mut runtime = build_solidity(
        "
        contract test {
            function do_test() public {
                uint8 x1 = 0;
                assert(~x1 == 255);
                int32 x2 = 0x7fefabcd;
                assert(uint32(~x2) == 0x80105432);
            }

            function do_complement(uint256 foo) public returns (uint) {
                return ~foo;
            }
        }",
    );

    runtime.function("do_test", Vec::new());

    let mut args = Vec::new();
    args.resize(32, 0);

    runtime.function("do_complement", args);

    let ret = runtime.vm.output;

    assert!(ret.len() == 32);
    assert!(ret.into_iter().filter(|x| *x == 255).count() == 32);
}

#[test]
fn bitwise() {
    // parse
    let mut runtime = build_solidity(
        "
        contract test {
            function do_test() public {
                uint8 x1 = 0xf0;
                uint8 x2 = 0x0f;
                assert(x1 | x2 == 0xff);
                assert(x1 ^ x2 == 0xff);
                assert(x1 & x2 == 0x00);
                assert(x1 ^ 0 == x1);

                int32 x3 = 0x7fefabcd;
                assert(x3 & 0xffff == 0xabcd);
            }

            function do_or(uint256 a, uint256 b) public returns (uint) {
                return a | b;
            }

            function do_and(uint256 a, uint256 b) public returns (uint) {
                return a & b;
            }

            function do_xor(uint256 a, uint256 b) public returns (uint) {
                return a ^ b;
            }
        }",
    );

    runtime.function("do_test", Vec::new());

    let mut args = Vec::new();
    args.resize(32, 0);
    args.resize(64, 0xff);

    runtime.function("do_xor", args);

    let ret = &runtime.vm.output;

    assert!(ret.len() == 32);
    assert!(ret.iter().filter(|x| **x == 255).count() == 32);

    let mut args = Vec::new();
    args.resize(32, 0);
    args.resize(64, 0xff);

    runtime.function("do_or", args);

    let ret = &runtime.vm.output;

    assert!(ret.len() == 32);
    assert!(ret.iter().filter(|x| **x == 255).count() == 32);

    let mut args = Vec::new();
    args.resize(32, 0);
    args.resize(64, 0xff);

    runtime.function("do_and", args);

    let ret = &runtime.vm.output;

    assert!(ret.len() == 32);
    assert!(ret.iter().filter(|x| **x == 0).count() == 32);
}

#[test]
fn shift() {
    // parse
    let mut runtime = build_solidity(
        "
        contract test {
            function do_test() public {
                uint8 x1 = 0xf0;
                uint8 x2 = 0x0f;
                assert(x1 >> 4 == 0x0f);
                assert(x2 << 4 == 0xf0);

                int x3 = -16;
                assert(x3 >> 2 == -4);

                uint x5 = 0xdead_0000_0000_0000_0000;
                assert(x5 >> 64 == 0xdead);

                x5 = 0xdead;
                assert(x5 << 64 == 0xdead_0000_0000_0000_0000);

            }
        }",
    );

    runtime.function("do_test", Vec::new());
}

#[test]
fn assign_bitwise() {
    // parse
    let mut runtime = build_solidity(
        "
        contract test {
            function do_test() public {
                uint8 x1 = 0xf0;
                uint8 x2 = 0x0f;
                x1 |= x2;
                assert(x1 == 0xff);
                x1 = 0xf0; x2 = 0x0f;
                x1 ^= x2;
                assert(x1 == 0xff);
                x1 = 0xf0; x2 = 0x0f;
                x1 &= x2;
                assert(x1 == 0x00);
                x1 = 0xf0; x2 = 0x0f;
                x1 ^= 0;
                assert(x1 == x1);

                int32 x3 = 0x7fefabcd;
                x3 &= 0xffff;
                assert(x3 == 0xabcd);
            }

            function do_or(uint256 a, uint256 b) public returns (uint) {
                a |= b;
                return a;
            }

            function do_and(uint256 a, uint256 b) public returns (uint) {
                a &= b;
                return a;
            }

            function do_xor(uint256 a, uint256 b) public returns (uint) {
                a ^= b;
                return a;
            }
        }",
    );

    runtime.function("do_test", Vec::new());

    let mut args = Vec::new();
    args.resize(32, 0);
    args.resize(64, 0xff);

    runtime.function("do_xor", args);

    let ret = &runtime.vm.output;

    assert!(ret.len() == 32);
    assert!(ret.iter().filter(|x| **x == 255).count() == 32);

    let mut args = Vec::new();
    args.resize(32, 0);
    args.resize(64, 0xff);

    runtime.function("do_or", args);

    let ret = &runtime.vm.output;

    assert!(ret.len() == 32);
    assert!(ret.iter().filter(|x| **x == 255).count() == 32);

    let mut args = Vec::new();
    args.resize(32, 0);
    args.resize(64, 0xff);

    runtime.function("do_and", args);

    let ret = &runtime.vm.output;

    assert!(ret.len() == 32);
    assert!(ret.iter().filter(|x| **x == 0).count() == 32);
}

#[test]
fn assign_shift() {
    // parse
    let mut runtime = build_solidity(
        "
        contract test {
            function do_test() public {
                uint8 x1 = 0xf0;
                uint8 x2 = 0x0f;
                x1 >>= 4;
                x2 <<= 4;
                assert(x1 == 0x0f);
                assert(x2 == 0xf0);

                int x3 = -16;
                x3 >>= 2;
                assert(x3 == -4);

                uint x5 = 0xdead_0000_0000_0000_0000;
                x5 >>= 64;
                assert(x5 == 0xdead);

                x5 = 0xdead;
                x5 <<= 64;
                assert(x5 == 0xdead_0000_0000_0000_0000);
            }
        }",
    );

    runtime.function("do_test", Vec::new());
}

#[test]
fn ternary() {
    // parse
    let mut runtime = build_solidity(
        "
        contract test {
            function do_test() public {
                uint8 x1 = 0xf0;
                uint8 x2 = 0x0f;

                assert((false ? x1 : x2) == x2);
                assert((true ? x1 : x2) == x1);
            }
        }",
    );

    runtime.function("do_test", Vec::new());
}

#[test]
fn short_circuit_or() {
    // parse
    let mut runtime = build_solidity(
        "
        contract test {
            uint32 counter;

            function increase_counter() private returns (bool) {
                counter += 1;
                return true;
            }

            function increase_counter2() private returns (bool) {
                counter++;
                return true;
            }

            function do_test() public {
                assert(counter == 0);

                // if left of or is true, right is not evaluated
                assert(true || increase_counter());
                assert(counter == 0);

                assert(false || increase_counter2());
                assert(counter == 1);

                false && increase_counter();
                assert(counter == 1);

                true && increase_counter();
                assert(counter == 2);
            }
        }",
    );

    runtime.function("do_test", Vec::new());
}

#[test]
fn short_circuit_and() {
    // parse
    let mut runtime = build_solidity(
        "
        contract test {
            uint32 counter;

            function increase_counter() private returns (bool) {
                counter |= 1;
                return false;
            }

            function increase_counter2() private returns (bool) {
                ++counter;
                return false;
            }

            function do_test() public {
                assert(counter == 0);

                increase_counter2();
                increase_counter2();

                assert(counter == 2);

                increase_counter();

                assert(counter == 3);

                counter = 0;

                // if left hand side is false, right hand side is not evaluated
                assert(!(false && increase_counter()));
                assert(counter == 0);
                assert(!(true && increase_counter2()));
                assert(counter == 1);
                false && increase_counter2();
                assert(counter == 1);
                counter = 0;
                true && increase_counter();
                assert(counter == 1);
            }
        }",
    );

    runtime.function("do_test", Vec::new());
}

#[test]
fn power() {
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct Val(u64);

    // parse
    let mut runtime = build_solidity(
        "
        contract c {
            function power(uint64 base, uint64 exp) public returns (uint64) {
                return base ** exp;
            }
        }",
    );

    // 4**5 = 1024
    let args = Val(4)
        .encode()
        .into_iter()
        .chain(Val(5).encode().into_iter())
        .collect();

    runtime.function("power", args);

    assert_eq!(runtime.vm.output, Val(1024).encode());

    // n ** 1 = n
    let args = Val(2345)
        .encode()
        .into_iter()
        .chain(Val(1).encode().into_iter())
        .collect();

    runtime.function("power", args);

    assert_eq!(runtime.vm.output, Val(2345).encode());

    // n ** 0 = 0
    let args = Val(0xdead_beef)
        .encode()
        .into_iter()
        .chain(Val(0).encode().into_iter())
        .collect();

    runtime.function("power", args);

    assert_eq!(runtime.vm.output, Val(1).encode());

    // 0 ** n = 0
    let args = Val(0)
        .encode()
        .into_iter()
        .chain(Val(0xdead_beef).encode().into_iter())
        .collect();

    runtime.function("power", args);

    assert_eq!(runtime.vm.output, Val(0).encode());

    let ns = parse_and_resolve(
        "contract test {
            function power(uint64 base, int64 exp) public returns (uint64) {
                return base ** exp;
            }
       }",
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "exponation (**) is not allowed with signed types"
    );

    let ns = parse_and_resolve(
        "contract test {
            function power(int64 base, uint64 exp) public returns (int64) {
                return base ** exp;
            }
       }",
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "exponation (**) is not allowed with signed types"
    );

    let ns = parse_and_resolve(
        "contract test {
            function power(int64 base, int64 exp) public returns (int64) {
                return base ** exp;
            }
       }",
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "exponation (**) is not allowed with signed types"
    );
}

#[test]
fn large_power() {
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct Val(u128);

    // parse
    let mut runtime = build_solidity(
        "
        contract c {
            function power(uint128 base, uint128 exp) public returns (uint128) {
                return base ** exp;
            }
        }",
    );

    // 4**5 = 1024
    let args = Val(4)
        .encode()
        .into_iter()
        .chain(Val(5).encode().into_iter())
        .collect();

    runtime.function("power", args);

    assert_eq!(runtime.vm.output, Val(1024).encode());

    // n ** 1 = n
    let args = Val(2345)
        .encode()
        .into_iter()
        .chain(Val(1).encode().into_iter())
        .collect();

    runtime.function("power", args);

    assert_eq!(runtime.vm.output, Val(2345).encode());

    // n ** 0 = 0
    let args = Val(0xdeadbeef)
        .encode()
        .into_iter()
        .chain(Val(0).encode().into_iter())
        .collect();

    runtime.function("power", args);

    assert_eq!(runtime.vm.output, Val(1).encode());

    // 0 ** n = 0
    let args = Val(0)
        .encode()
        .into_iter()
        .chain(Val(0xdeadbeef).encode().into_iter())
        .collect();

    runtime.function("power", args);

    assert_eq!(runtime.vm.output, Val(0).encode());

    // 10 ** 36 = 1000000000000000000000000000000000000
    let args = Val(10)
        .encode()
        .into_iter()
        .chain(Val(36).encode().into_iter())
        .collect();

    runtime.function("power", args);

    assert_eq!(
        runtime.vm.output,
        Val(1000000000000000000000000000000000000).encode()
    );
}

#[test]
fn multiply() {
    let mut rng = rand::thread_rng();
    let size = 32;

    let mut runtime = build_solidity(
        "
        contract c {
            function multiply(uint a, uint b) public returns (uint) {
                return a * b;
            }
        }",
    );

    let mut rand = || -> (BigInt, Vec<u8>) {
        let length = rng.gen::<usize>() % size;

        let mut data = Vec::new();

        data.resize_with(length + 1, || rng.gen());

        data.resize(size, 0);

        (BigInt::from_bytes_le(Sign::Plus, &data), data)
    };

    for _ in 0..1000 {
        let (a, a_data) = rand();
        let (b, b_data) = rand();

        println!("in: a:{:?} b:{:?}", a_data, b_data);

        runtime.function(
            "multiply",
            a_data.into_iter().chain(b_data.into_iter()).collect(),
        );

        println!("out: res:{:?}", runtime.vm.output);

        let res = BigInt::from_bytes_le(Sign::Plus, &runtime.vm.output);

        println!("{} = {} * {}", res, a, b);

        // the result is truncated to $size bytes. We do this here by converting to Vec<u8> and truncating
        // it. A truncating bigint multiply would be nicer.
        let (_, mut res) = (a * b).to_bytes_le();
        res.resize(size, 0);

        assert_eq!(res, runtime.vm.output);
    }
}

#[test]
fn bytes_bitwise() {
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct Bytes3([u8; 3]);
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct Bytes5([u8; 5]);
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct BytesArray([u8; 7], u32);

    // parse
    let mut runtime = build_solidity(
        "
        contract c {
            function or(bytes5 x) public returns (bytes5 y) {
                y = x | hex\"80808080\";
            }

            function and(bytes5 x) public returns (bytes5) {
                return x & hex\"FFFF\";
            }

            function xor(bytes5 x) public returns (bytes5) {
                x ^= hex\"FF00\";

                return x;
            }

            function shift_left(bytes3 a) public returns (bytes5 b) {
                b = bytes5(a) << 8;
            }

            function shift_right(bytes3 a) public returns (bytes5 b) {
                b = bytes5(a) >> 8;
            }

            function shift_left2(bytes3 a) public returns (bytes5 b) {
                b = bytes5(a);
                b <<= 8;
            }

            function shift_right2(bytes3 a) public returns (bytes5 b) {
                b = bytes5(a);
                b >>= 8;
            }

            function bytes_length() public {
                bytes4 b4;

                assert(b4.length == 4);
            }

            function complement(bytes3 a) public returns (bytes3) {
                return ~a;
            }

            function bytes_array(bytes7 foo, uint32 index) public returns (bytes1) {
                return foo[index];
            }
        }",
    );

    runtime.function("or", Bytes5([0x01, 0x01, 0x01, 0x01, 0x01]).encode());

    assert_eq!(
        runtime.vm.output,
        Bytes5([0x81, 0x81, 0x81, 0x81, 0x01]).encode()
    );

    runtime.function("and", Bytes5([0x01, 0x01, 0x01, 0x01, 0x01]).encode());

    assert_eq!(runtime.vm.output, Bytes5([0x01, 0x01, 0, 0, 0]).encode());

    runtime.function("xor", Bytes5([0x01, 0x01, 0x01, 0x01, 0x01]).encode());

    assert_eq!(
        runtime.vm.output,
        Bytes5([0xfe, 0x01, 0x01, 0x01, 0x01]).encode()
    );

    // shifty-shift
    runtime.function("shift_left", Bytes3([0xf3, 0x7d, 0x03]).encode());

    assert_eq!(
        runtime.vm.output,
        Bytes5([0x7d, 0x03, 0x00, 0x00, 0x00]).encode()
    );

    runtime.function("shift_right", Bytes3([0xf3, 0x7d, 0x03]).encode());

    assert_eq!(
        runtime.vm.output,
        Bytes5([0x00, 0xf3, 0x7d, 0x03, 0x00]).encode()
    );

    // assignment versions
    runtime.function("shift_left2", Bytes3([0xf3, 0x7d, 0x03]).encode());

    assert_eq!(
        runtime.vm.output,
        Bytes5([0x7d, 0x03, 0x00, 0x00, 0x00]).encode()
    );

    runtime.function("shift_right2", Bytes3([0xf3, 0x7d, 0x03]).encode());

    assert_eq!(
        runtime.vm.output,
        Bytes5([0x00, 0xf3, 0x7d, 0x03, 0x00]).encode()
    );

    // check length
    runtime.function("bytes_length", Vec::new());

    // complement
    runtime.function("complement", Bytes3([0xf3, 0x7d, 0x03]).encode());

    assert_eq!(runtime.vm.output, Bytes3([0x0c, 0x82, 0xfc]).encode());

    // array access
    let bytes7 = *b"NAWABRA";
    for i in 0..6 {
        runtime.function("bytes_array", BytesArray(bytes7, i).encode());

        assert_eq!(runtime.vm.output, [bytes7[i as usize]]);
    }
}

#[test]
#[should_panic]
fn bytesn_underflow_index_acccess() {
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct BytesArray([u8; 7], i32);

    // parse
    let mut runtime = build_solidity(
        "
        contract test {
            function bytes_array(bytes7 foo, int32 index) public returns (bytes1) {
                return foo[index];
            }
        }",
    );

    runtime.function("bytes_array", BytesArray(*b"nawabra", -1).encode());
}

#[test]
#[should_panic]
fn bytesn_overflow_index_acccess() {
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct BytesArray([u8; 7], i32);

    // parse
    let mut runtime = build_solidity(
        "
        contract test {
            function bytes_array(bytes7 foo, int32 index) public returns (byte) {
                return foo[index];
            }
        }",
    );

    runtime.function("bytes_array", BytesArray(*b"nawabra", 7).encode());
}

#[test]
fn unaryminus_and_subtract() {
    // The minus sign can be a unary negative or subtract.
    let mut runtime = build_solidity(
        r#"
        contract c {
            function test() public {
                uint32 x = 10-10;
                assert(x == 0);
                int32 y = -10-10;
                assert(y == -20);
            }
        }"#,
    );

    runtime.function("test", Vec::new());
}

#[test]
fn div() {
    // The minus sign can be a unary negative or subtract.
    let mut runtime = build_solidity(
        r#"
        contract c {
            function test1() public {
                // see https://solidity.readthedocs.io/en/latest/types.html#modulo
                assert(int256(5) % int256(2) == int256(1));
                assert(int256(5) % int256(-2) == int256(1));
                assert(int256(-5) % int256(2) == int256(-1));
                assert(int256(-5) % int256(-2) == int256(-1));

                assert(int64(5) % int64(2) == int64(1));
                assert(int64(5) % int64(-2) == int64(1));
                assert(int64(-5) % int64(2) == int64(-1));
                assert(int64(-5) % int64(-2) == int64(-1));
            }

            function test2() public {
                // see https://github.com/hyperledger/burrow/pull/1367#issue-399914366
                assert(int256(7) / int256(3) == int256(2));
                assert(int256(7) / int256(-3) == int256(-2));
                assert(int256(-7) / int256(3) == int256(-2));
                assert(int256(-7) / int256(-3) == int256(2));

                assert(int256(7) % int256(3) == int256(1));
                assert(int256(7) % int256(-3) == int256(1));
                assert(int256(-7) % int256(3) == int256(-1));
                assert(int256(-7) % int256(-3) == int256(-1));

                assert(int64(7) / int64(3) == int64(2));
                assert(int64(7) / int64(-3) == int64(-2));
                assert(int64(-7) / int64(3) == int64(-2));
                assert(int64(-7) / int64(-3) == int64(2));

                assert(int64(7) % int64(3) == int64(1));
                assert(int64(7) % int64(-3) == int64(1));
                assert(int64(-7) % int64(3) == int64(-1));
                assert(int64(-7) % int64(-3) == int64(-1));
            }
        }"#,
    );

    runtime.function("test1", Vec::new());

    runtime.function("test2", Vec::new());
}

#[test]
fn destructure() {
    let ns = parse_and_resolve(
        "contract test {
            function foo(uint bar) public {
                int a;
                int b;

                (a, b) = (1, 2, 3);
            }
        }",
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "destructuring assignment has 2 elements on the left and 3 on the right"
    );

    let ns = parse_and_resolve(
        "contract test {
            function foo(uint bar) public {
                int a;
                int b;

                (c, b) = (1, 2);
            }
        }",
        Target::Substrate,
    );

    assert_eq!(first_error(ns.diagnostics), "`c\' is not found");

    let ns = parse_and_resolve(
        "contract test {
            function foo(uint bar) public {
                int a;
                int b;

                (a memory, b) = (1, 2);
            }
        }",
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "storage modifier ‘memory’ not permitted on assignment"
    );

    let ns = parse_and_resolve(
        "contract test {
            function foo(uint bar) public {
                int a;
                int b;

                (a , b) = (1, );
            }
        }",
        Target::Substrate,
    );

    assert_eq!(first_error(ns.diagnostics), "stray comma");

    // The minus sign can be a unary negative or subtract.
    let mut runtime = build_solidity(
        r#"
        contract c {
            function test() public {
                int a;
                int b;

                // test one
                (a, b) = (102, 3);

                assert(b == 3 && a == 102);

                // test missing one
                (a, , b) = (1, 2, 3);

                assert(a == 1 && b == 3);

                // test single one
                (a) = 5;

                assert(a == 5);

                // or like so
                (a) = (105);

                assert(a == 105);
            }

            function swap() public {
                int32 a;
                int32 b;

                // test one
                (a, b) = (102, 3);

                // test swap
                (b, a) = (a, b);

                assert(a == 3 && b == 102);
            }
        }"#,
    );

    runtime.function("test", Vec::new());

    runtime.function("swap", Vec::new());

    // The minus sign can be a unary negative or subtract.
    let mut runtime = build_solidity(
        r#"
        contract c {
            function test() public {
                // test one
                (int32 a, int32 b) = (102, 3);

                assert(b == 3 && a == 102);

                // test missing one
                (a, , b) = (1, 2, 3);

                assert(a == 1 && b == 3);

                // test single one
                (a) = 5;

                assert(a == 5);

                // or like so
                (a) = (105);

                assert(a == 105);
            }
        }"#,
    );

    runtime.function("test", Vec::new());
}
