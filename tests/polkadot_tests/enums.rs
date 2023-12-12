// SPDX-License-Identifier: Apache-2.0

use parity_scale_codec::{Decode, Encode};

use crate::build_solidity;

#[test]
fn weekdays() {
    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    struct Val(u8);

    // parse
    let mut runtime = build_solidity(
        "
        enum Weekday { Monday, Tuesday, Wednesday, Thursday, Friday, Saturday, Sunday }

        contract enum_example {
            function is_weekend(Weekday day) public pure returns (bool) {
                return (day == Weekday.Saturday || day == Weekday.Sunday);
            }

            function test_values() public pure {
                assert(int8(Weekday.Monday) == 0);
                assert(int8(Weekday.Tuesday) == 1);
                assert(int8(Weekday.Wednesday) == 2);
                assert(int8(Weekday.Thursday) == 3);
                assert(int8(Weekday.Friday) == 4);
                assert(int8(Weekday.Saturday) == 5);
                assert(int8(Weekday.Sunday) == 6);

                Weekday x;
                x = Weekday.Monday;

                assert(uint(x) == 0);

                x = Weekday.Sunday;
                assert(int16(x) == 6);

                x = Weekday(2);
                assert(x == Weekday.Wednesday);

                assert(type(Weekday).max == 6);
                assert(type(Weekday).min == 0);

                // stray type() does not do anything.
                type(Weekday);
            }
        }",
    );

    runtime.function("is_weekend", Val(4).encode());

    assert_eq!(runtime.output(), Val(0).encode());

    runtime.function("is_weekend", Val(5).encode());

    assert_eq!(runtime.output(), Val(1).encode());

    runtime.function("test_values", Vec::new());
}

#[test]
fn enums_other_contracts() {
    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    struct Val(u8);

    // parse
    let mut runtime = build_solidity(
        "
        contract a {
            c.foo bar;

            constructor() public {
                bar = c.foo.bar;
            }

            function test(c.foo x) public {
                assert(x == c.foo.bar2);
                assert(c.foo.bar2 != c.foo.bar3);
            }
        }

        abstract contract c {
            enum foo { bar, bar2, bar3 }
        }
        ",
    );

    runtime.function("test", Val(1).encode());
}
