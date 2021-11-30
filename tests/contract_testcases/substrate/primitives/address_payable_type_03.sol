
        contract c {
            function test(address payable a) public {
                other b = a;
            }
        }

        contract other {
            function test() public {
            }
        }