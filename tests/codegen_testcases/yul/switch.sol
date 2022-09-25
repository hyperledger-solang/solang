// RUN: --target solana --emit cfg 

contract Testing {
    // BEGIN-CHECK: Testing::Testing::function::switch_default__uint256
    function switch_default(uint a) public pure returns (uint b) {
        assembly {
            // CHECK: switch (arg #0):
            switch a
            // CHECK: case uint256 1: goto block #2
            // CHECK: case uint256 2: goto block #3
            // CHECK: default: goto block #4

            // CHECK: block1: # end_switch
            // CHECK: branchcond (%b == uint256 7), block5, block6
            case 1 {
                // CHECK: block2: # case_0
                // CHECK: ty:uint256 %b = uint256 5
                b := 5
                // CHECK: branch block1
            }
            case 2 {
                // CHECK: block3: # case_1
                // CHECK: ty:uint256 %b = uint256 6
                b := 6
                // CHECK: branch block1
            }
            default {
                // CHECK: block4: # default
                // CHECK: ty:uint256 %b = uint256 7
                b := 7
                // CHECK: branch block1
            }
        }

        if (b == 7) {
            b += 1;
        }
    }

    // BEGIN-CHECK: Testing::Testing::function::switch_no_default__uint256
    function switch_no_default(uint a) public pure returns (uint b) {
        assembly {
            switch a
            // CHECK: switch (arg #0):
		    // CHECK: case uint256 1: goto block #2
		    // CHECK: case uint256 2: goto block #3
		    // CHECK: default: goto block #1

            // CHECK: block1: # end_switch
	        // CHECK: branchcond (%b == uint256 5), block4, block5

            case 1 {
            // CHECK: block2: # case_0
            // CHECK: ty:uint256 %b = uint256 5
            // CHECK: branch block1
                b := 5
            }
            case 2 {
            // CHECK: block3: # case_1
            // CHECK: ty:uint256 %b = uint256 6
	        // CHECK: branch block1
                b := 6
            }
        }

        if (b == 5) {
            b += 1;
        }
    }
}
