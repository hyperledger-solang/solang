
        contract c {
            function test(address payable a) public {
                other b = a;
            }
        }

        contract other {
            function test() public {
            }
        }
// ----
// error (102-103): implicit conversion to contract other from address payable not allowed
