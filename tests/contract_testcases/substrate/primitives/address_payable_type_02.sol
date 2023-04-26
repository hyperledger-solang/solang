
        contract c {
            function test(address a) public {
                other b = a;
            }
        }

        contract other {
            function test() public {
            }
        }
// ----
// error (94-95): implicit conversion to contract other from address not allowed
