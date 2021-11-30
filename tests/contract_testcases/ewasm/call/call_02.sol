
        contract x {
            function f(address payable a) public {
                (bool s, bytes memory bs) = a.call{value: 2}("");
            }
        }
        