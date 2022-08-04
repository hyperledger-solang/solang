// SPDX-License-Identifier: Apache-2.0

use crate::build_solidity;
use parity_scale_codec::{Decode, Encode};

#[test]

fn array_boundary_check() {
    #[derive(Encode, Decode)]
    struct SetArg(u32);

    #[derive(Encode, Decode)]
    struct BooleanArg(bool);

    let mut contract = build_solidity(
        r#"
        contract Array_bound_Test {
            function array_bound() public pure {
                uint256[] a = new uint256[](10);
                uint256 sesa = 0;
                if (1 > 2) {
                    a.push(5);
                } else {
                    a.push(1);
                }

                for (uint256 i = 0; i < a.length; i++) {
                    sesa = sesa + a[10];
                }

                assert(sesa == 11);
            }
        }
        "#,
    );

    contract.function("array_bound", Vec::new());

    let mut contract = build_solidity(
        r#"
        contract Array_bound_Test {
            function array_bound(uint32 size32) public {
                uint256[] c = new uint256[](size32);
                uint256[] d = new uint256[](20);
                uint32 sesa = c.length + d.length;

                assert(sesa == 31);
            }
        }
    "#,
    );

    contract.function("array_bound", SetArg(11).encode());

    let mut contract = build_solidity(
        r#"
        contract c {
            function test(bool cond) public returns (uint32) {
                bool[] b = new bool[](100);
                if (cond) {
                    b.push(true);
                }

                assert(b.length == 101);

                if (cond) {
                    b.pop();
                    b.pop();
                }

                assert(b.length == 99);
                return b.length;
            }
        }

    "#,
    );

    contract.function("test", BooleanArg(true).encode());

    let mut contract = build_solidity(
        r#"
        contract c {
            function test_for_loop() public {
                uint256[] a = new uint256[](20);
                a.push(1);
                uint256 sesa = 0;

                for (uint256 i = 0; i < a.length; i++) {
                    sesa = sesa + a[20];
                }

                assert(sesa == 21);
            }
        }

    "#,
    );

    contract.function("test_for_loop", Vec::new());

    let mut contract = build_solidity(
        r#"
        contract c {
            function test_loop_2() public  {
                int256[] vec = new int256[](10);

                for (int256 i = 0; i < 5; i++) {
                    if (vec.length > 20) {
                        break;
                    }
                    vec.push(3);
                }

                assert (vec.length == 15);

            }
        }

    "#,
    );

    contract.function("test_loop_2", Vec::new());

    let mut contract = build_solidity(
        r#"
        contract foo {
            function fool() public {
                int256[] a = new int256[](3);
                // copy by reference/pointer
                int256[] b = a;

                b = new int256[](6);

                assert(a.length == 3);
                assert(b.length == 6);
            }

            function fool2() public {
                int256[] a = new int256[](3);
                // copy by reference/pointer
                int256[] b = a;
                a.pop();
                assert(a.length == b.length);
                assert(a.length == 2);
                assert(b.length == 2);
                // now both a and b have length 2.
            }
        }

    "#,
    );

    contract.function("fool", Vec::new());
    contract.function("fool2", Vec::new());

    let mut contract = build_solidity(
        r#"
        contract c {
            function nested_assign() public pure {
                uint32[] a;
                uint32 b = (a = new uint32[](4))[0];
                a.pop();
                assert(a.length == 3);
            }

            function edgy() public pure {
                uint256[] a;
                uint256[] b;
                uint256[] c = new uint256[](30);

                a = b = c = new uint256[](40);
                a.pop();

                assert(a.length == b.length);
                assert(b.length == c.length);
                assert(c.length == 39);
            }

            function edgy_2() public pure {
                uint256[] a;
                uint256[] b;
                uint256[] c = new uint256[](30);

                a = b = c;
                a.pop();

                assert(a.length == b.length);
                assert(b.length == c.length);
                assert(c.length == 29);
            }
        }


    "#,
    );

    contract.function("nested_assign", Vec::new());
    contract.function("edgy", Vec::new());
    contract.function("edgy_2", Vec::new());
}
