// RUN: --target solana --emit cfg -Onone --no-cse

contract testing {

    // BEGIN-CHECK: testing::testing::function::calls_and_unreachable__uint64_uint64
    function calls_and_unreachable(uint64 a, uint64 b) public pure {
        assembly {
            function test1(c : u128, d) {
                let y := mul(c, d)
            }

            // CHECK:  = call testing::yul_function_0::test1 (zext uint128 (arg #0)), (zext uint256 (arg #1))
            test1(a, b)

            // CHECK: assert-failure
            invalid()

            // NOT-CHECK: ty:uint256 %x =
            let x := mul(a, b)
        }
    }

    // BEGIN-CHECK: testing::testing::function::yul_block__uint16_int16
    function yul_block(uint16 a, int16 b) public pure {
        assembly {
            // CHECK: ty:uint256 %x = (sext uint256 ((sext int24 (arg #1)) >> (zext int24 (arg #0))))
            let x := shr(a, b)
            {
                // CHECK: ty:uint256 %y = (overflowing (sext uint256 ((sext int24 (arg #1)) >> (zext int24 (arg #0)))) * (zext uint256 (arg #0)))
                let y := mul(x, a)

                // CHECK: ty:uint8 %g = uint8((unsigned less int256((overflowing (sext uint256 ((sext int24 (arg #1)) >> (zext int24 (arg #0)))) * (zext uint256 (arg #0)))) < (sext int256 (arg #1))))
                let g : u8 := lt(y, b)
            }
        }
    }

    // BEGIN-CHECK: testing::testing::function::variable_declaration__bool_uint8
    function variable_declaration(bool a, uint8 b) public pure {
        assembly {
            function multiple() -> ret1, ret2 : s32 {
                ret1 := 4
                ret2 := 6
            }

            function unique() -> ret3 {
                ret3 := 6
            }

            // CHECK: %ret3.temp.40 = call testing::yul_function_2::unique
            // CHECK: ty:uint256 %c = %ret3.temp.40
            let c := unique()

            // CHECK: %ret1.temp.41, %ret2.temp.42 = call testing::yul_function_1::multiple
            // CHECK: ty:uint256 %d = %ret1.temp.41
            // CHECK: ty:int8 %e = (trunc int8 %ret2.temp.42)
            let d, e : s8 := multiple()

            // CHECK: ty:uint256 %f = uint256(((arg #0) == bool 0))
            let f := iszero(a)


            // CHECK: ty:uint256 %g = undef
            // CHECK: ty:uint256 %h = undef
            // CHECK: ty:uint256 %i = undef
            let g, h, i
        }
    }

// BEGIN-CHECK: testing::testing::function::if_statement__bool
    function if_statement(bool a) public pure {
        assembly {
            let x := 5
            // CHECK: branchcond (arg #0), block1, block2
            if a {
            // CHECK: block1: # then
            // CHECK: ty:uint256 %x = (overflowing uint256((arg #0)) + uint256 5)
                x := add(a, x)

                // CHECK: ty:bool %a = false
                a := false
            // CHECK: branch block2
            }
            // CHECK: block2: # endif

            // CHECK: branchcond (uint256 0 != %x), block3, block4
            if x {
            // CHECK: block3: # then

            // CHECK: ty:uint256 %x = uint256 5
                x := 5
            // CHECK: branch block4
            }
            // CHECK: block4: # endif

            // CHECK: branchcond (unsigned less %x < uint256 3), block5, block6
            if lt(x, 3) {
            // CHECK: block5: # then

            // CHECK: ty:uint256 %x = uint256 4
                x := 4
            // CHECK: branch block6
            }
            // CHECK: block6: # endif

            // CHECK: branchcond (unsigned less %x < uint256 3), block7, block8
            if lt(x, 3) {
            // CHECK: block7: # then
            // CHECK: ty:uint256 %x = uint256 6
                x := 6
            // CHECK: assert-failure
                invalid()
            // NOT-CHECK: branch
            }

            // CHECK: branchcond (%x == uint256 3), block9, block10
            if eq(x, 3) {
                // CHECK: block9: # then
                // CHECK: branch block10
            }

            // CHECK: block10: # endif
            // CHECK: ty:uint256 %x = uint256 4
            x := 4

        }
    }

// BEGIN-CHECK: testing::testing::function::for_statement__bool
    function for_statement(bool a) public pure {
        assembly {

            for {
            // CHECK: ty:uint256 %i = uint256 1
                let i := 1
            // CHECK: branch block1
            // CHECK: block1: # cond
            // CHECK: branchcond (uint256 0 != (overflowing %i + uint256 1)), block3, block4
            } add(i, 1) {
                // CHECK: block2: # next
                // CHECK: ty:uint256 %i = (overflowing %i - uint256 1)
                i := sub(i, 1)
                // CHECK: branch block1
            } {
                // CHECK: block3: # body
                // CHECK: ty:uint256 %i = (%i << uint256 1)
                i := mul(i, 2)
                // CHECK: branch block2
            }
            // CHECK: block4: # end_for

            for {
                // CHECK: ty:uint256 %i.27 = uint256 1
                // CHECK: branch block5
                let i := 1
                // CHECK: block5: # cond
                // CHECK: branchcond (unsigned less %i.27 < uint256 10), block7, block8
            } lt(i, 10) {
                // CHECK: block6: # next
                i := add(i, 1)
                // CHECK: ty:uint256 %i.27 = (overflowing %i.27 + uint256 1)
                // CHECK: branch block5
            } {
                // CHECK: block7: # body
                // CHECK: ty:uint256 %i.27 = (uint256 2 >> %i.27)
                i := shr(i, 2)
                // CHECK: branch block6
            }
            // CHECK: block8: # end_for

            for {
                // CHECK: ty:uint256 %i.28 = uint256 1
                let i := 1
                // CHECK: branch block9
                // CHECK: block9: # cond
                // CHECK: branchcond %a, block11, block12
            } a {
                // CHECK: block10: # next
                // CHECK: ty:uint256 %i.28 = (overflowing uint256(%a) + uint256 1)
                i := add(a, 1)
                // CHECK: ty:bool %a = false
                a := false
            } {
                // CHECK: block11: # body
                // CHECK: ty:bool %a = (uint256 0 != (overflowing %i.28 + uint256 2))
                a := add(i, 2)
                // CHECK: branch block10
            }
            // CHECK: block12: # end_for

            for {
                // CHECK: ty:uint256 %i.29 = uint256 2
                let i := 2
                // CHECK: branch block13
                // CHECK: block13: # cond
                // CHECK: branchcond (uint256 2 == uint256 0), block15, block16
            } eq(i, 0) {
                // CHECK: block14: # next
                // NOT-CHECK: ty:uint256 %i.29 =
                i := sub(i, 2)
            } {
                // CHECK: block15: # body
                i := add(i, 3)
                // CHECK: ty:uint256 %i.29 = uint256 5
                invalid()
                // CHECK: assert-failure
                // NOT-CHECK: branch
            }
            // CHECK: block16: # end_for

            for {
                // CHECK: ty:uint256 %j = uint256 2
                let j := 2
                // CHECK: branch block17
                // CHECK: block17: # cond
                // CHECK: branchcond (uint256 2 == uint256 3), block19, block20
            } eq(j, 3) {
                // CHECK: block18: # next
                j := shr(j, 2)
                // CHECK: ty:uint256 %j = (uint256 2 >> %j)
                invalid()
                // CHECK: assert-failure
            } {
                // CHECK: block19: # body
                j := sar(j, 3)
                // CHECK: branch block18
            }
            // CHECK: block20: # end_for

            for {
                // CHECK: ty:uint256 %i.31 = uint256 0
                let i := 0
                // CHECK: branch block21
                // CHECK: block21: # cond
                // CHECK: branchcond (unsigned less %i.31 < uint256 10), block23, block24
            } lt(i, 10) {
                // CHECK: block22: # next
                i := add(i, 1)
                // CHECK: ty:uint256 %i.31 = (overflowing %i.31 + uint256 1)
                // CHECK: branch block21
            } {
                // CHECK: block23: # body
                for {
                    // CHECK: ty:uint256 %j.32 = uint256 0
                    let j :=0
                    // CHECK: branch block25
// ---- block 24 contains the for-loop with the invalid function
// CHECK: block24: # end_for
// CHECK: ty:uint256 %i.33 = uint256 2
// CHECK: assert-failure
// NOT-CHECK: branch

                    // CHECK: block25: # cond
                    // CHECK: branchcond (unsigned less %j.32 < uint256 10), block27, block28
                } lt(j, 10) {
                    // CHECK: ty:uint256 %j.32 = (overflowing %j.32 + uint256 1)
                    j := add(j, 1)
                    // CHECK: branch block25
                } {
                    // CHECK: block27: # body
                    // CHECK: ty:bool %a = (uint256 0 != (overflowing %i.31 + %j.32))
                    a := add(i, j)
                    // CHECK: branch block26
                }
                // CHECK: block28: # end_for
                // CHECK: branch block22
            }

            for {
                let i := 2
                invalid()
            } lt(3, 4) {
                i := add(i, 1)
            } {
                i := sub(i, 2)
            }

        }
    }

// BEGIN-CHECK: testing::testing::function::break_statement
    function break_statement() public pure {
        assembly {
            for {let i := 1
            // CHECK: ty:uint256 %i = uint256 1
            // CHECK: branch block1
            // CHECK: block1: # cond
            // CHECK: branchcond (unsigned less %i < uint256 10), block3, block4
            } lt(i, 10) {i := add(i, 1)
            // CHECK: block2: # next
            // CHECK: ty:uint256 %i = (overflowing %i + uint256 1)
            // CHECK: branch block1
            } {
                // CHECK: block3: # body
                i := shr(i, 2)
                // CHECK: ty:uint256 %i = (uint256 2 >> %i)
                // CHECK: branchcond (unsigned more %i > uint256 10), block5, block6
                if gt(i, 10) {
                    break
                }
            }
            // CHECK: block4: # end_for
            // CHECK: return

            // IF-block:
            // CHECK: block5: # then
            // CHECK: branch block4

            // End of for for loop after IF
            // CHECK: block6: # endif
            // CHECK: branch block2
        }
    }

    // BEGIN-CHECK: testing::testing::function::break_nested_for
    function break_nested_for() public pure {
        assembly {
            for {
                // CHECK: ty:uint256 %i = uint256 1
                let i := 1
                // CHECK: branch block1
                // CHECK: branchcond (unsigned less %i < uint256 10), block3, block4
            } lt(i, 10) {
                // CHECK: block2: # next
                i := add(i, 1)
                // CHECK: ty:uint256 %i = (overflowing %i + uint256 1)
                // CHECK: branch block1
            } {
                for {
                    // CHECK: block3: # body
                    let j := 2
                    // CHECK: ty:uint256 %j = uint256 2
                    // CHECK: branch block5
                } lt(j, 10) {
                    // after outer for:
                    // CHECK: block4: # end_for
                    // CHECK: return

                    // inner for condition
                    // CHECK: block5: # cond
                    // CHECK: branchcond (unsigned less %j < uint256 10), block7, block8
                    // CHECK: block6: # next
                    // CHECK: ty:uint256 %j = (overflowing %j + uint256 1)
                    // CHECK: branch block5
                    j := add(j, 1)
                } {
                    // CHECK: block7: # body
                    // CHECK: branchcond (unsigned more %j > uint256 5), block9, block10
                    if gt(j, 5) {
                        break
                    }
                    // After inner for:
                    // CHECK: block8: # end_for
                    // CHECK: branchcond (unsigned more %i > uint256 5), block11, block12

                    // Inside inner if:
                    // CHECK: block9: # then
                    // CHECK: branch block8

                    // After inner if:
                    // CHECK: block10: # endif
                    // CHECK: ty:uint256 %j = (overflowing %i - uint256 2)
                    j := sub(i, 2)
                    // CHECK: branch block6
                }
                if gt(i, 5) {
                    // CHECK: block11: # then
                    break
                    // CHECK: branch block4
                }
                // CHECK: block12: # endif
                // CHECK: ty:uint256 %i = (overflowing %i - uint256 4)
                i := sub(i, 4)
                // CHECK: branch block2
            }
        }
    }

    // BEGIN-CHECK: testing::testing::function::continue_statement
    function continue_statement() public pure {
        assembly {
            for {let i := 1
            // CHECK: ty:uint256 %i = uint256 1
            // CHECK: branch block1
            // CHECK: block1: # cond
            // CHECK: branchcond (unsigned less %i < uint256 10), block3, block4
            } lt(i, 10) {i := add(i, 1)
            // CHECK: block2: # next
            // CHECK: ty:uint256 %i = (overflowing %i + uint256 1)
            // CHECK: branch block1
            } {
                // CHECK: block3: # body
                i := shr(i, 2)
                // CHECK: ty:uint256 %i = (uint256 2 >> %i)
                // CHECK: branchcond (unsigned more %i > uint256 10), block5, block6
                if gt(i, 10) {
                    continue
                }
            }
            // CHECK: block4: # end_for
            // CHECK: return

            // IF-block:
            // CHECK: block5: # then
            // CHECK: branch block2

            // End of for for loop after IF
            // CHECK: block6: # endif
            // CHECK: branch block2
        }
    }

    // BEGIN-CHECK: testing::testing::function::continue_nested_for
    function continue_nested_for() public pure {
        assembly {
            for {
                // CHECK: ty:uint256 %i = uint256 1
                let i := 1
                // CHECK: branch block1
                // CHECK: branchcond (unsigned less %i < uint256 10), block3, block4
            } lt(i, 10) {
                // CHECK: block2: # next
                i := add(i, 1)
                // CHECK: ty:uint256 %i = (overflowing %i + uint256 1)
                // CHECK: branch block1
            } {
                for {
                    // CHECK: block3: # body
                    let j := 2
                    // CHECK: ty:uint256 %j = uint256 2
                    // CHECK: branch block5
                } lt(j, 10) {
                    // after outer for:
                    // CHECK: block4: # end_for
                    // CHECK: return

                    // inner for condition
                    // CHECK: block5: # cond
                    // CHECK: branchcond (unsigned less %j < uint256 10), block7, block8
                    // CHECK: block6: # next
                    // CHECK: ty:uint256 %j = (overflowing %j + uint256 1)
                    // CHECK: branch block5
                    j := add(j, 1)
                } {
                    // CHECK: block7: # body
                    // CHECK: branchcond (unsigned more %j > uint256 5), block9, block10
                    if gt(j, 5) {
                        continue
                    }
                    // After inner for:
                    // CHECK: block8: # end_for
                    // CHECK: branchcond (unsigned more %i > uint256 5), block11, block12

                    // Inside inner if:
                    // CHECK: block9: # then
                    // CHECK: branch block6

                    // After inner if:
                    // CHECK: block10: # endif
                    // CHECK: ty:uint256 %j = (overflowing %i - uint256 2)
                    j := sub(i, 2)
                    // CHECK: branch block6
                }
                if gt(i, 5) {
                    // CHECK: block11: # then
                    continue
                    // CHECK: branch block2
                }
                // CHECK: block12: # endif
                // CHECK: ty:uint256 %i = (overflowing %i - uint256 4)
                i := sub(i, 4)
                // CHECK: branch block2
            }
        }
    }
}