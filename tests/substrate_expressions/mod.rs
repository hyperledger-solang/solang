use parity_scale_codec::Encode;
use parity_scale_codec_derive::{Encode, Decode};

use super::{build_solidity, first_error};
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
fn expressions() {
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct Val16(u16);
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct Val8(u8);

    // parse
    let (runtime, mut store) = build_solidity("
        contract test {
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
        }",
    );

    runtime.function(&mut store, "add_100", Val16(0xffc0).encode());

    assert_eq!(store.scratch, Val16(36).encode());

    runtime.function(&mut store, "clear_digit", Val8(25).encode());

    assert_eq!(store.scratch, Val8(20).encode());

    runtime.function(&mut store, "low_digit", Val8(25).encode());

    assert_eq!(store.scratch, Val8(5).encode());
}