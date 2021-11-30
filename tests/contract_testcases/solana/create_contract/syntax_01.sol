
        contract y {
            function f() public {
                x a = new x{salt: 102}();
            }
        }
        contract x {}
    