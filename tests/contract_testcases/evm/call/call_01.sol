
        contract x {
            function f(address payable a) public {
                (bool s, bytes memory bs) = a.staticcall{value: 2}("");
            }
        }
        