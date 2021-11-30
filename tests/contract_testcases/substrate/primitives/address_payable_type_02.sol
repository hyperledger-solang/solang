
        contract c {
            function test(address a) public {
                other b = a;
            }
        }

        contract other {
            function test() public {
            }
        }