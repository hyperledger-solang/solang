use parity_scale_codec::Encode;
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

            function bar() public {
                assert(foo(10) == 5960);
            }
        }",
    );

    runtime.function(&mut store, "bar", Vec::new());
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