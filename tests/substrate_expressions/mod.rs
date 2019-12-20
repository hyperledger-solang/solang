use parity_scale_codec::{Encode, Decode};
use parity_scale_codec_derive::{Encode, Decode};

use super::{build_solidity, first_error, no_errors};
use solang::{parse_and_resolve, Target};

#[test]
fn celcius_and_fahrenheit() {
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct Val(u32);

    // parse
    let (runtime, mut store) = build_solidity("
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

    runtime.function(&mut store, "celcius2fahrenheit", Val(10).encode());

    assert_eq!(store.scratch, Val(50).encode());

    runtime.function(&mut store, "fahrenheit2celcius", Val(50).encode());

    assert_eq!(store.scratch, Val(10).encode());
}

#[test]
fn digits() {
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct Val32(u32);
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct Val64(u64);

    // parse
    let (runtime, mut store) = build_solidity("
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

    runtime.function(&mut store, "digitslen", Val64(1234567).encode());

    assert_eq!(store.scratch, Val32(7).encode());

    runtime.function(&mut store, "sumdigits", Val64(123456789).encode());

    assert_eq!(store.scratch, Val32(45).encode());
}

#[test]
fn large_loops() {
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct Val32(u32);
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct Val64(u64);

    // parse
    let (runtime, mut store) = build_solidity("
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

    runtime.function(&mut store, "bar", Vec::new());

    let mut args = Val64(7000).encode();
    args.resize(32, 0);

    runtime.function(&mut store, "baz", args);

    let mut rets = Val64(7000_000_000).encode();
    rets.resize(32, 0);

    assert_eq!(store.scratch, rets);
}

#[test]
fn expressions() {
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct Val16(u16);
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct Val8(u8);

    // parse
    let (runtime, mut store) = build_solidity("
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

    runtime.function(&mut store, "add_100", Val16(0xffc0).encode());

    assert_eq!(store.scratch, Val16(36).encode());

    runtime.function(&mut store, "clear_digit", Val8(25).encode());

    assert_eq!(store.scratch, Val8(20).encode());

    runtime.function(&mut store, "low_digit", Val8(25).encode());

    assert_eq!(store.scratch, Val8(5).encode());

    runtime.function(&mut store, "test_comparisons", Vec::new());

    runtime.function(&mut store, "increments", Vec::new());
}

#[test]
fn test_cast_errors() {
    let (_, errors) = parse_and_resolve(
        "contract test {
            function foo(uint bar) public {
                bool is_nonzero = bar;
            }
        }", &Target::Substrate);

    assert_eq!(first_error(errors), "conversion from uint256 to bool not possible");

    let (_, errors) = parse_and_resolve(
        "contract test {
            function foobar(uint foo, int bar) public returns (bool) {
                return (foo < bar);
            }
        }", &Target::Substrate);

    assert_eq!(first_error(errors), "implicit conversion would change sign from uint256 to int256");

    let (_, errors) = parse_and_resolve(
        "contract test {
            function foobar(int32 foo, uint16 bar) public returns (bool) {
                foo = bar;
                return false;
            }
        }", &Target::Substrate);

    no_errors(errors);

    // int16 can be negative, so cannot be stored in uint32
    let (_, errors) = parse_and_resolve(
        "contract test {
            function foobar(uint32 foo, int16 bar) public returns (bool) {
                foo = bar;
                return false;
            }
        }", &Target::Substrate);

    assert_eq!(first_error(errors), "implicit conversion would change sign from int16 to uint32");

    let (_, errors) = parse_and_resolve(
        "contract foo {
            uint bar;

            function set_bar(uint32 b) public {
                bar = b;
            }

            function get_bar() public returns (uint32) {
                return uint32(bar);
            }
        }

        contract bar {
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
        }", &Target::Substrate);

    no_errors(errors);
}

#[test]
#[should_panic]
fn divisions_by_zero() {
    // parse
    let (runtime, mut store) = build_solidity("
        contract test {
            function do_test() public returns (uint){
                uint256 val = 100;

                return (val / 0);
            }
        }",
    );

    runtime.function(&mut store, "do_test", Vec::new());
}

#[test]
fn divisions() {
    // parse
    let (runtime, mut store) = build_solidity("
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

    runtime.function(&mut store, "do_test", Vec::new());
}

#[test]
fn divisions64() {
    // parse
    let (runtime, mut store) = build_solidity("
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

    runtime.function(&mut store, "do_test", Vec::new());
}

#[test]
fn divisions128() {
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct Args(i128, i128);

    #[derive(Debug, PartialEq, Encode, Decode)]
    struct Rets(i128);

    // parse
    let (runtime, mut store) = build_solidity("
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

    runtime.function(&mut store, "do_test", Vec::new());

    runtime.function(&mut store, "return_neg", Vec::new());

    if let Ok(Rets(r)) = Rets::decode(&mut &store.scratch[..]) {
        assert_eq!(r, -100);
    } else {
        assert!(false);
    }

    runtime.function(&mut store, "return_pos", Vec::new());

    if let Ok(Rets(r)) = Rets::decode(&mut &store.scratch[..]) {
        assert_eq!(r, 255);
    } else {
        assert!(false);
    }

    runtime.function(&mut store, "do_div", Args(-9900, -100).encode());

    if let Ok(Rets(r)) = Rets::decode(&mut &store.scratch[..]) {
        assert_eq!(r, 99);
    } else {
        assert!(false);
    }

    runtime.function(&mut store, "do_div", Args(-101213131318098987, -100000).encode());

    if let Ok(Rets(r)) = Rets::decode(&mut &store.scratch[..]) {
        assert_eq!(r, 1012131313180);
    } else {
        assert!(false);
    }

    runtime.function(&mut store, "do_signed_test", Vec::new());
}

#[test]
fn divisions256() {
    // parse
    let (runtime, mut store) = build_solidity("
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

    runtime.function(&mut store, "do_test", Vec::new());
}

#[test]
fn complement() {
    // parse
    let (runtime, mut store) = build_solidity("
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

    runtime.function(&mut store, "do_test", Vec::new());

    let mut args = Vec::new();
    args.resize(32, 0);

    runtime.function(&mut store, "do_complement", args);

    let ret = store.scratch;

    assert!(ret.len() == 32);
    assert!(ret.into_iter().filter(|x| *x == 255).count() == 32);
}

#[test]
fn bitwise() {
    // parse
    let (runtime, mut store) = build_solidity("
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

    runtime.function(&mut store, "do_test", Vec::new());

    let mut args = Vec::new();
    args.resize(32, 0);
    args.resize(64, 0xff);

    runtime.function(&mut store, "do_xor", args);

    let ret = &store.scratch;

    assert!(ret.len() == 32);
    assert!(ret.iter().filter(|x| **x == 255).count() == 32);

    let mut args = Vec::new();
    args.resize(32, 0);
    args.resize(64, 0xff);

    runtime.function(&mut store, "do_or", args);

    let ret = &store.scratch;

    assert!(ret.len() == 32);
    assert!(ret.iter().filter(|x| **x == 255).count() == 32);

    let mut args = Vec::new();
    args.resize(32, 0);
    args.resize(64, 0xff);

    runtime.function(&mut store, "do_and", args);

    let ret = &store.scratch;

    assert!(ret.len() == 32);
    assert!(ret.iter().filter(|x| **x == 0).count() == 32);
}

#[test]
fn shift() {
    // parse
    let (runtime, mut store) = build_solidity("
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

    runtime.function(&mut store, "do_test", Vec::new());
}

#[test]
fn assign_bitwise() {
    // parse
    let (runtime, mut store) = build_solidity("
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

    runtime.function(&mut store, "do_test", Vec::new());

    let mut args = Vec::new();
    args.resize(32, 0);
    args.resize(64, 0xff);

    runtime.function(&mut store, "do_xor", args);

    let ret = &store.scratch;

    assert!(ret.len() == 32);
    assert!(ret.iter().filter(|x| **x == 255).count() == 32);

    let mut args = Vec::new();
    args.resize(32, 0);
    args.resize(64, 0xff);

    runtime.function(&mut store, "do_or", args);

    let ret = &store.scratch;

    assert!(ret.len() == 32);
    assert!(ret.iter().filter(|x| **x == 255).count() == 32);

    let mut args = Vec::new();
    args.resize(32, 0);
    args.resize(64, 0xff);

    runtime.function(&mut store, "do_and", args);

    let ret = &store.scratch;

    assert!(ret.len() == 32);
    assert!(ret.iter().filter(|x| **x == 0).count() == 32);
}

#[test]
fn assign_shift() {
    // parse
    let (runtime, mut store) = build_solidity("
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

    runtime.function(&mut store, "do_test", Vec::new());
}

#[test]
fn ternary() {
    // parse
    let (runtime, mut store) = build_solidity("
        contract test {
            function do_test() public {
                uint8 x1 = 0xf0;
                uint8 x2 = 0x0f;

                assert((false ? x1 : x2) == x2);
                assert((true ? x1 : x2) == x1);
            }
        }",
    );

    runtime.function(&mut store, "do_test", Vec::new());
}
