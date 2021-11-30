
        contract c {
            function test(address payable a) public {
                other b = other(a);
            }
        }

        contract other {
            function test() public {
            }
        }