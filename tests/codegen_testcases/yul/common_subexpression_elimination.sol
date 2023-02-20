// RUN: --target solana --emit cfg

contract testing  {
    // BEGIN-CHECK: testing::testing::function::general_test__uint64
    function general_test(uint64 a) public view returns (uint64, uint256) {
        uint64 g = 0;
        uint256 h = 0;
        assembly {
            function sum(a, b) -> ret1 {
                ret1 := add(a, b)
            }

            function mix(a, b) -> ret1, ret2 {
                ret1 := mul(a, b)
                ret2 := add(a, b)
            }

            // CHECK: block1: # cond
            // CHECK: ty:uint256 %1.cse_temp = (zext uint256 (arg #0))
            for {let i := 0} lt(i, 10) {i := add(i, 1)} {
                // CHECK: block3: # body
                // CHECK: branchcond (%1.cse_temp == uint256 259), block5, block6
                if eq(a, 259) {
                    break
                }

                // This is the if-condition after the loop
                // block4: # end_for
                // CHECK: branchcond ((unsigned less %1.cse_temp < uint256 10) | (%1.cse_temp == uint256 259)), block9, block10
                g := sum(g, 2)
                // CHECK: block6: # endif
                // CHECK: branchcond (unsigned more %1.cse_temp > uint256 10), block7, block8
                if gt(a, 10) {
                    continue
                }
                g := sub(g, 1)
            }

            if or(lt(a, 10), eq(a, 259)) {
                g, h := mix(g, 10)
            }
        }

        return (g, h);
    }
}