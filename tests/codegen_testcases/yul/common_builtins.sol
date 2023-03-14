// RUN: --target solana --emit cfg -Onone --no-cse

contract testing {
    // BEGIN-CHECK: testing::testing::function::not_isZero__uint64
    function not_isZero(uint64 a) public pure {
        assembly {
            // CHECK: ty:uint256 %x = (zext uint256 ~(arg #0))
            let x := not(a)

            // CHECK: ty:bool %y = ((arg #0) == uint64 0)
            let y : bool := iszero(a)
        }
    }

// BEGIN-CHECK: testing::testing::function::addMod_mulMod__uint64_uint64_uint64
    function addMod_mulMod(uint64 a, uint64 b, uint64 c) public pure {
        assembly {
            let x := addmod(a, b, c)
            // CHECK: branchcond ((arg #2) == uint64 0), block1, block2
            // CHECK: block1: # then
            // CHECK: ty:uint256 %temp.11 = uint256 0
            // CHECK: branch block3
            // CHECK: block2: # else
            // CHECK: ty:uint256 %temp.11 = (builtin AddMod ((arg #1), (arg #0), (arg #2)))
            // CHECK: branch block3
            // CHECK: block3: # endif
            // CHECK: # phis: temp.11
            // CHECK: ty:uint256 %x = %temp.11

            let y :s32  := mulmod(a, b, c)
            // CHECK: branchcond ((arg #2) == uint64 0), block4, block5
            // CHECK: block4: # then
            // CHECK: ty:uint256 %temp.12 = uint256 0
            // CHECK: branch block6
            // CHECK: block5: # else
            // CHECK: ty:uint256 %temp.12 = (builtin MulMod ((arg #1), (arg #0), (arg #2)))
            // CHECK: branch block6
            // CHECK: block6: # endif
            // CHECK: # phis: temp.12
            // CHECK: ty:int32 %y = (trunc int32 %temp.12)
        }
    }

// BEGIN-CHECK: testing::testing::function::byte_builtin__int64_uint256
    function byte_builtin(int64 a, uint256 b) public pure {
        assembly {
            let x := byte(b, a)
            // CHECK: branchcond (unsigned (arg #1) >= uint256 32), block1, block2
            // CHECK: block1: # then
            // CHECK: ty:uint256 %temp.13 = uint256 0
            // CHECK: branch block3
            // CHECK: block2: # else
            // CHECK: ty:uint256 %temp.13 = (((sext uint256 (arg #0)) >> ((uint256 31 - (arg #1)) << uint256 3)) & uint256 255)
            // CHECK: branch block3
            // CHECK: block3: # endif
            // CHECK: # phis: temp.13
            // CHECK: ty:uint256 %x = %temp.13
        }
    }
}