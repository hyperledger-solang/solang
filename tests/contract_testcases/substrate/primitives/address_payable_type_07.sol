
        contract c {
            function test(address a) public {
                address payable b = address payable(a);
            }
        }