
        contract c {
            function test(address payable a) public {
                other b = a;
            }
        }

        contract other {
            function test() public {
            }
        }
// ---- Expect: diagnostics ----
// error: 4:27-28: implicit conversion to contract other from address payable not allowed
