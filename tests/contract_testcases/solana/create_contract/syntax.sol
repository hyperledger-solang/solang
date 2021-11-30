
        contract y {
            function f() public {
                x a = new x{gas: 102}();
            }
        }
        contract x {}
    