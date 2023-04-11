// RUN: --target solana --emit cfg

contract foo {
    // BEGIN-CHECK: foo::foo::function::test
    function test(uint x) public {
        uint256 yy=0;
        assembly {
        // Ensure the CSE temp is not before the switch
        // CHECK: ty:uint256 %y = uint256 5
	    // CHECK: switch ((arg #0) & uint256 3):
            let y := 5

            switch and(x, 3)
                case 0 {
                    y := 5
                    x := 5
                }
                case 1 {
                    y := 7
                    x := 9
                }
                case 3 {
                    y := 10
                    x := 80
                }

            // CHECK: block1: # end_switch
	        // CHECK: ty:uint256 %1.cse_temp = (overflowing %x + %y)
	        // CHECK: branchcond (%1.cse_temp == uint256 90), block5, block6
            if eq(add(x, y), 90) {
                yy := 9
            }

            // CHECK: branchcond (%1.cse_temp == uint256 80), block7, block8
            if eq(add(x, y), 80) {
                yy := 90
            }
        }
    }
}