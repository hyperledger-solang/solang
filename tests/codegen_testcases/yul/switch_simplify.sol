// RUN: --target solana --emit cfg

contract test {
    // BEGIN-CHECK: test::test::function::test_1
    function test_1() public pure returns (int) {
        int gg = 56;

        int res = 0;
        assembly {
            // NOT-CHECK: switch
            // CHECK: branch block3
            switch add(gg, 4)
            case 5 {
                res := 90
            }
            case 60 {
                // CHECK: block3: # case_1
	            // CHECK: ty:int256 %res = int256 4
                res := 4
            }
            default {
                res := 7
            }
        }

        return res;
    }

    // BEGIN-CHECK: test::test::function::test_2
    function test_2() public pure returns (int) {
        int gg = 56;

        int res = 0;
        assembly {
            // NOT-CHECK: switch
            // CHECK: branch block4
            switch add(gg, 4)
            case 5 {
                res := 90
            }
            case 6 {
                res := 4
            }
            default {
                // CHECK: block4: # default
	            // CHECK: ty:int256 %res = int256 7
                res := 7
            }
        }

        return res;
    }

    // BEGIN-CHECK: test::test::function::test_3
    function test_3() public pure returns (int) {
        int gg = 56;

        int res = 0;
        assembly {
            // NOT-CHECK: switch
            // CHECK: branch block1
            switch add(gg, 4)
            case 5 {
                res := 90
            }
            case 6 {
                res := 4
            }
        }

        // CHECK: block1: # end_switch
	    // CHECK: return %res

        return res;
    }
}