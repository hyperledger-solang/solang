// SPDX-License-Identifier: Apache-2.0

use crate::build_solidity;

#[test]
fn using_for_contracts() {
    let mut runtime = build_solidity(
        r#"
        interface I {
            function f(int) external;
        }

        library L {
            function F(I i, bool b, int n) public {
                if (b) {
                    print("Hello");
                }
            }
        }

        contract C {
            using L for I;

            function test() public {
                I i = I(address(0));

                i.F(true, 102);
            }
        }"#,
    );

    runtime.constructor(&[]);
    runtime.function("test", &[]);

    assert_eq!(runtime.logs, "Hello");

    let mut runtime = build_solidity(
        r#"
        interface I {
            function f1(int) external;
            function X(int) external;
        }

        library L {
            function f1_2(I i) external {
                i.f1(2);
            }

            function X(I i) external {
                print("X lib");
            }
        }

        contract foo is I {
            using L for I;

            function test() public {
                I i = I(address(this));

                i.X();
                i.X(2);
                i.f1_2();
            }

            function f1(int x) public {
                print("x:{}".format(x));
            }

            function X(int) public {
                print("X contract");
            }
        }"#,
    );

    runtime.constructor(&[]);
    runtime.function("test", &[]);

    assert_eq!(runtime.logs, "X libX contractx:2");
}

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

    runtime.constructor(&[]);

    runtime.function("test_cmp", &[]);
    runtime.function("test_arith", &[]);
    runtime.function("test_bit", &[]);
}
