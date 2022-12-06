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

    runtime.constructor("C", &[]);
    runtime.function("test", &[], None);

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

    runtime.constructor("foo", &[]);
    runtime.function("test", &[], None);

    assert_eq!(runtime.logs, "X libX contractx:2");
}
