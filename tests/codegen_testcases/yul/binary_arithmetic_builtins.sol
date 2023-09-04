
// RUN: --target solana --emit cfg -Onone --no-cse
contract testing {
// BEGIN-CHECK: testing::testing::function::add_sub_mul__int16_int32_uint256_uint128
    function add_sub_mul(int16 a, int32 b, uint256 c, uint128 d) public pure {
        assembly {
            // CHECK: ty:uint256 %e = (sext uint256 (overflowing (sext int32 (arg #0)) + (arg #1)))
            let e := add(a, b)

            // CHECK: ty:uint256 %f = uint256((overflowing (sext int256 (arg #1)) + int256((arg #2))))
            let f := add(b, c)

            // CHECK: ty:uint256 %g = (overflowing (arg #2) + (zext uint256 (arg #3)))
            let g := add(c, d)

            // CHECK: ty:uint256 %h = (sext uint256 (overflowing (sext int136 (arg #0)) + (zext int136 (arg #3))))
            let h := add(a, d)

            // CHECK: ty:uint256 %i = uint256((overflowing (sext int256 (arg #0)) + int256((arg #2))))
            let i := add(a, c)

            // CHECK: ty:uint256 %j = (sext uint256 (overflowing (sext int136 (arg #1)) + (zext int136 (arg #3))))
            let j := add(b, d)

            // CHECK: ty:int32 %k = (overflowing (sext int32 (arg #0)) - (arg #1))
            let k : s32 := sub(a, b)

            // CHECK: ty:int256 %l = int256((overflowing (arg #2) - (zext uint256 (arg #3))))
            let l : s256 := sub(c, d)

            // CHECK: ty:uint256 %m = (overflowing (arg #2) * (zext uint256 (arg #3)))
            let m := mul(c, d)

            // CHECK: ty:uint256 %n = uint256 43981
            let n := hex"abcd"

            // CHECK: ty:uint256 %o = uint256 1667327589
            let o := "cafe"

            // CHECK: ty:uint256 %p = uint256 73330734691809
            let p := mul(n, o)

            // CHECK: ty:uint256 %q = uint256 22193982385802470
            let q := mul("cofe", hex"caffee")

            // CHECK: ty:bool %r = false
            let r : bool := false

            // CHECK: ty:bool %s = true
            let s : bool := true

            // CHECK: ty:uint256 %t = (overflowing uint256 22193982385802470 + uint256(false))
            let t := add(q, r)

            // CHECK: ty:uint256 %u = uint256 115792089237316195423570985008687907853269984665640564039457584007913129639935
            let u := sub(false, true)

            // CHECK: ty:uint256 %v = uint256((overflowing true + false))
            let v := add(s, r)
        }
    }

// BEGIN-CHECK:  testing::testing::function::op_that_branch__uint256_uint256_int256_int256
    function op_that_branch(uint256 a, uint256 b, int256 c, int256 d) public view {
        assembly {
            let e := div(a, b)
            // CHECK: branchcond ((arg #1) == uint256 0), block1, block2
            // CHECK: block1: # then
            // CHECK: ty:uint256 %temp.49 = uint256 0
            // CHECK: branch block3
            // CHECK: block2: # else
            // CHECK: ty:uint256 %temp.49 = (unsigned divide (arg #0) / (arg #1))
            // CHECK: branch block3
            // CHECK: block3: # endif
            // CHECK: # phis: temp.49
            // CHECK: ty:uint256 %e = %temp.49

            let f := sdiv(c, d)
            // CHECK: branchcond ((arg #3) == int256 0), block4, block5
            // CHECK: block4: # then
            // CHECK: ty:uint256 %temp.50 = uint256 0
            // CHECK: branch block6
            // CHECK: block5: # else
            // CHECK: ty:uint256 %temp.50 = (signed divide (arg #2) / (arg #3))
            // CHECK: branch block6
            // CHECK: block6: # endif
            // CHECK: # phis: temp.50
            // CHECK: ty:uint256 %f = %temp.50


            let g := mod(a, b)
            // CHECK: branchcond ((arg #1) == uint256 0), block7, block8
            // CHECK: block7: # then
            // CHECK: ty:uint256 %temp.51 = uint256 0
            // CHECK: branch block9
            // CHECK: block8: # else
            // CHECK: ty:uint256 %temp.51 = (unsigned modulo (arg #0) % (arg #1))
            // CHECK: branch block9
            // CHECK: block9: # endif
            // CHECK: # phis: temp.51
            // CHECK: ty:uint256 %g = %temp.51

            let h := smod(c, d)
            // CHECK: branchcond ((arg #3) == int256 0), block10, block11
            // CHECK: block10: # then
            // CHECK: ty:uint256 %temp.52 = uint256 0
            // CHECK: branch block12
            // CHECK: block11: # else
            // CHECK: ty:uint256 %temp.52 = (signed modulo (arg #2) % (arg #3))
            // CHECK: branch block12
            // CHECK: block12: # endif
            // CHECK: # phis: temp.52
            // CHECK: ty:uint256 %h = %temp.52
        }
    }

// BEGIN-CHECK: testing::testing::function::exponential__int128_int8
    function exponential(int128 a, int8 b) public pure {
        assembly {
            // CHECK: ty:uint256 %x = (sext uint256 (overflowing (arg #0) ** (sext int128 (arg #1))))
            let x := exp(a, b)
        }
    }

// BEGIN-CHECK: testing::testing::function::compare__uint64_uint64_int64_int64
    function compare(uint64 a, uint64 b, int64 c, int64 d) public pure {
        assembly {
            // CHECK: ty:bool %e = (unsigned less (arg #0) < (arg #1))
            let e : bool := lt(a, b)

            // CHECK: ty:uint8 %f = uint8((unsigned more (arg #0) > (arg #1)))
            let f : u8 := gt(a, b)

            // CHECK: ty:int8 %h = int8((signed less (arg #2) < (arg #3)))
            let h : s8 := slt(c, d)

            // CHECK: ty:uint256 %i = uint256((signed more (arg #0) > (arg #1)))
            let i := sgt(a, b)

            // CHECK: ty:uint256 %j = uint256(((zext int72 (arg #0)) == (sext int72 (arg #3))))
            let j := eq(a, d)
        }
    }

// BEGIN-CHECK: testing::testing::function::bitwise_op__uint256_int256
    function bitwise_op(uint256 a, int256 b) public pure {
        assembly {
            // CHECK: ty:uint256 %c = uint256((int256((arg #0)) | (arg #1)))
            let c := or(a, b)

            // CHECK: ty:uint256 %d = uint256(((arg #1) ^ int256((arg #0))))
            let d := xor(b, a)

            // CHECK: ty:uint256 %e = uint256(((arg #1) << int256((arg #0))))
            let e := shl(a, b)

            // CHECK: ty:uint256 %f = uint256((int256((arg #0)) >> (arg #1)))
            let f := shr(b, a)

            // CHECK: ty:uint256 %g = uint256(((arg #1) >> int256((arg #0))))
            let g := sar(a, b)
        }
    }
}