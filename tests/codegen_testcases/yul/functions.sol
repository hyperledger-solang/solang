// RUN: --target solana --emit cfg -Onone --no-cse

contract testing {
    function yul_function() public pure returns (uint256) {
        uint256 c;
        uint256 d;
        assembly {
            // BEGIN-CHECK: # function testing::yul_function_0::early_leave public:false selector:00000000 nonpayable:true
            // CHECK: # params: uint256,uint256
            // CHECK: # returns:
            function early_leave(a, b) {
                let x := add(a, b)
                if lt(x, 2) {
                    // CHECK: block1: # then
                    x := sub(a, b)
                    // CHECK: ty:uint256 %x = ((arg #0) - (arg #1))
                    leave
                    // CHECK: return
                }
                // CHECK: block2: # endif
                // CHECK: ty:uint256 %x = (uint256 2 << (arg #0))
                x := shl(a, 2)
                // CHECK: return
            }

            // BEGIN-CHECK: function testing::yul_function_1::single_return public:false selector:00000000 nonpayable:true
            // CHECK: # params: uint256,int32
            // CHECK: # returns: uint256
            function single_return(a, b : s32) -> ret1 {
                if lt(a, 2) {
                    // CHECK: block1: # then
                    ret1 := add(sub(a,b), mul(shr(a, 2), 3))
                    // CHECK: ty:uint256 %ret1 = uint256(((int256((arg #0)) - (sext int256 (arg #1))) + int256(((uint256 2 >> (arg #0)) * uint256 3))))
                    leave
                    // CHECK: return %ret1
                }
                // CHECK: block2: # endif
                ret1 := a
                // CHECK: ty:uint256 %ret1 = (arg #0)
                // CHECK: return (arg #0)
            }

            // BEGIN-CHECK: function testing::yul_function_2::multiple_returns public:false selector:00000000 nonpayable:true
            // CHECK: # params: uint256,uint256
            // CHECK: # returns: uint64,uint256
            function multiple_returns(a, b) -> ret1 : u64, ret2 {
                if lt(a, 2) {
                // CHECK: block1: # then
                    ret1 := a
                // CHECK: ty:uint64 %ret1 = (trunc uint64 (arg #0))
                    ret2 := b
                // CHECK: ty:uint256 %ret2 = (arg #1)
                    leave
                // CHECK: return (trunc uint64 (arg #0)), (arg #1)
                }
                // CHECK: ty:uint64 %ret1 = (trunc uint64 (arg #1))
                ret1 := b
                // CHECK: ty:uint256 %ret2 = (arg #0)
                ret2 := a
                // CHECK: return (trunc uint64 (arg #1)), (arg #0)
            }

            c := 1
            d := 2
            early_leave(c, d)

            c := single_return(c, d)
            c, d := multiple_returns(c, d)            
        }

        return c+d;
    }
}