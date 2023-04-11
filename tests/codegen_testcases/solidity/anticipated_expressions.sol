// RUN: --target solana --emit cfg

contract Test {

    // BEGIN-CHECK: Test::Test::function::test1__int256_int256
    function test1(int a, int b) pure public returns (int) {
        // CHECK: ty:int256 %1.cse_temp = ((arg #0) + (arg #1))
	    // CHECK: ty:int256 %x = %1.cse_temp
	    // CHECK: ty:int256 %3.cse_temp = ((arg #0) - (arg #1))
	    // CHECK: ty:int256 %z = %3.cse_temp
        int x = a + b;
        int z = a-b;
        int p=0;

        // CHECK: block1: # cond
        // CHECK: ty:int256 %2.cse_temp = (signed modulo (arg #0) % (arg #1))
        while(x != 0) {
        // CHECK: block2: # body
        // CHECK: ty:int256 %z = (%z + int256 9)
	    // CHECK: ty:int256 %y = %1.cse_temp
	    // CHECK: ty:int256 %x = (%x - %1.cse_temp)
            z+=9;
            int y = a + b;
            x -= y;
        // CHECK: block3: # endwhile
        // CHECK: ty:int256 %p2 = %2.cse_temp
	    // CHECK: return ((((%x + int256 9) - %z) + %p) - (int256 2 * %2.cse_temp))

            if (x == 9) {
                // CHECK: block4: # then
                // CHECK: ty:int256 %y = %3.cse_temp
	            // CHECK: ty:int256 %p = %2.cse_temp
	            // CHECK: ty:int256 %x = (%x + %3.cse_temp)
                y = a - b;
                p = a % b;
                x += y;
            }
        }

        int p2 = a%b;
        return x+9-z + p - 2*p2;
    }

    // BEGIN-CHECK: Test::Test::function::test2__int256_int256
    function test2(int a, int b) public pure returns (int) {
        int y = a-b;
        int j=0;
        int k=0;
        int l=0;
        int m=0;
        // CHECK: ty:int256 %1.cse_temp = ((arg #0) + (arg #1))
        if(y == 5) {
            // CHECK: block1: # then
            // CHECK: ty:int256 %j = %1.cse_temp
            j = a+b;
            // CHECK: block3: # endif
            // CHECK: ty:int256 %n = %1.cse_temp
            // CHECK: return ((((%j + %k) + %l) + %m) + %1.cse_temp)
        } else if (y == 2) {
            // CHECK: block4: # then
            // CHECK: ty:int256 %k = %1.cse_temp
            k = a+b;
        } else if (y == 3) {
            // CHECK: block7: # then
            // CHECK: ty:int256 %l = %1.cse_temp
            l = a+b;
        } else {
            // CHECK: block8: # else
            // CHECK: ty:int256 %m = %1.cse_temp
            m = a+b;
        }

        int n = a+b;
        return j+k+l+m+n;
    }

    // BEGIN-CHECK: Test::Test::function::test3__int256_int256
    function test3(int a, int b) public pure returns (int) {
        int y = a-b;
        int j=0;
        int k=0;
        int l=0;
        int m=0;
        // NOT-CHECK: ty:int256 %1.cse_temp
        if(y == 5) {
            // CHECK: block1: # then
            // CHECK: ty:int256 %j = ((arg #0) + (arg #1))
            j = a+b;
            // CHECK: block3: # endif
            // CHECK: ty:int256 %n = (%a + (arg #1))
	        // CHECK: return ((((%j + %k) + %l) + %m) + %n)
        } else if (y == 2) {
            // CHECK: block4: # then
	        // CHECK: ty:int256 %a = int256 9
	        // CHECK: ty:int256 %k = (int256 9 + (arg #1))
            a = 9;
            k = a+b;
        } else if (y == 3) {
            // CHECK: block7: # then
	        // CHECK: ty:int256 %l = ((arg #0) + (arg #1))
            l = a+b;
        } else {
            // CHECK: block8: # else
	        // CHECK: ty:int256 %m = ((arg #0) + (arg #1))
            m = a+b;
        }

        int n = a+b;
        return j+k+l+m+n;
    }

    // BEGIN-CHECK: Test::Test::function::test4__int256_int256
    function test4(int a, int b) public pure returns (int) {
        int y = a-b;
        int j=0;
        int k=0;
        int l=0;
        int m=0;
        // CHECK: ty:int256 %1.cse_temp = (overflowing (arg #0) * (arg #1))
        // CHECK: block1: # end_switch
	    // CHECK: ty:int256 %m = %1.cse_temp
	    // CHECK: return (%l + %1.cse_temp)
        assembly {
            switch y
                case 1 {
                    // CHECK: block2: # case_0
	                // CHECK: ty:int256 %j = %1.cse_temp
                    j := mul(a, b)
                }
                case 2 {
                    // CHECK: block3: # case_1
	                // CHECK: ty:int256 %k = %1.cse_temp
                    k := mul(a, b)
                }
                default {
                    // CHECK: block4: # default
	                // CHECK: ty:int256 %l = %1.cse_temp
                    l := mul(a, b)
                }
        }

        unchecked {
            m = a*b;
        }

        return l+m;
    }
}