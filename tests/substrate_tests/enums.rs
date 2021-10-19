use parity_scale_codec::Encode;
use parity_scale_codec_derive::{Decode, Encode};

use crate::{build_solidity, first_error, no_errors, parse_and_resolve};
use solang::Target;

#[test]
fn weekdays() {
    #[derive(Debug, PartialEq, Encode, Decode)]
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
            }
        }",
    );

    runtime.function("is_weekend", Val(4).encode());

    assert_eq!(runtime.vm.output, Val(0).encode());

    runtime.function("is_weekend", Val(5).encode());

    assert_eq!(runtime.vm.output, Val(1).encode());

    runtime.function("test_values", Vec::new());
}

#[test]
fn enums_other_contracts() {
    #[derive(Debug, PartialEq, Encode, Decode)]
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

        contract c {
            enum foo { bar, bar2, bar3 }
        }
        ",
    );

    runtime.function("test", Val(1).encode());
}

#[test]
fn test_cast_errors() {
    let ns = parse_and_resolve(
        "contract test {
            enum state { foo, bar, baz }
            function foo() public pure returns (uint8) {
                return state.foo;
            }
        }",
        Target::Substrate {
            address_length: 32,
            value_length: 16,
        },
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "implicit conversion from enum test.state to uint8 not allowed"
    );

    let ns = parse_and_resolve(
        "contract test {
            enum state {  }
            function foo() public pure returns (uint8) {
                return state.foo;
            }
        }",
        Target::Substrate {
            address_length: 32,
            value_length: 16,
        },
    );

    assert_eq!(first_error(ns.diagnostics), "enum ‘state’ has no fields");

    let ns = parse_and_resolve(
        "contract test {
            enum state { foo, bar, baz }
            function foo() public pure returns (uint8) {
                return uint8(state.foo);
            }
        }",
        Target::Substrate {
            address_length: 32,
            value_length: 16,
        },
    );

    no_errors(ns.diagnostics);
}

#[test]
fn incorrect_fields() {
    let ns = parse_and_resolve(
        "enum state { }",
        Target::Substrate {
            address_length: 32,
            value_length: 16,
        },
    );

    assert_eq!(first_error(ns.diagnostics), "enum ‘state’ has no fields");

    let ns = parse_and_resolve(
        r#"enum bar {
        foo0, foo1, foo2, foo3, foo4, foo5, foo6, foo7, foo8, foo9, foo10, foo11,
        foo12, foo13, foo14, foo15, foo16, foo17, foo18, foo19, foo20, foo21,
        foo22, foo23, foo24, foo25, foo26, foo27, foo28, foo29, foo30, foo31,
        foo32, foo33, foo34, foo35, foo36, foo37, foo38, foo39, foo40, foo41, foo42,
        foo43, foo44, foo45, foo46, foo47, foo48, foo49, foo50, foo51, foo52, foo53,
        foo54, foo55, foo56, foo57, foo58, foo59, foo60, foo61, foo62, foo63, foo64,
        foo65, foo66, foo67, foo68, foo69, foo70, foo71, foo72, foo73, foo74, foo75,
        foo76, foo77, foo78, foo79, foo80, foo81, foo82, foo83, foo84, foo85, foo86,
        foo87, foo88, foo89, foo90, foo91, foo92, foo93, foo94, foo95, foo96, foo97,
        foo98, foo99, foo100, foo101, foo102, foo103, foo104, foo105, foo106, foo107,
        foo108, foo109, foo110, foo111, foo112, foo113, foo114, foo115, foo116,
        foo117, foo118, foo119, foo120, foo121, foo122, foo123, foo124, foo125,
        foo126, foo127, foo128, foo129, foo130, foo131, foo132, foo133, foo134,
        foo135, foo136, foo137, foo138, foo139, foo140, foo141, foo142, foo143,
        foo144, foo145, foo146, foo147, foo148, foo149, foo150, foo151, foo152,
        foo153, foo154, foo155, foo156, foo157, foo158, foo159, foo160, foo161,
        foo162, foo163, foo164, foo165, foo166, foo167, foo168, foo169, foo170,
        foo171, foo172, foo173, foo174, foo175, foo176, foo177, foo178, foo179,
        foo180, foo181, foo182, foo183, foo184, foo185, foo186, foo187, foo188,
        foo189, foo190, foo191, foo192, foo193, foo194, foo195, foo196, foo197,
        foo198, foo199, foo200, foo201, foo202, foo203, foo204, foo205, foo206,
        foo207, foo208, foo209, foo210, foo211, foo212, foo213, foo214, foo215,
        foo216, foo217, foo218, foo219, foo220, foo221, foo222, foo223, foo224,
        foo225, foo226, foo227, foo228, foo229, foo230, foo231, foo232, foo233,
        foo234, foo235, foo236, foo237, foo238, foo239, foo240, foo241, foo242,
        foo243, foo244, foo245, foo246, foo247, foo248, foo249, foo250, foo251,
        foo252, foo253, foo254, foo255, foo256
        }"#,
        Target::Substrate {
            address_length: 32,
            value_length: 16,
        },
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "enum ‘bar’ has 257 fields, which is more than the 256 limit"
    );
}
