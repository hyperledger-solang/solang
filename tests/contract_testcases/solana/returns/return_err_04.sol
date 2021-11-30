
        contract foo {
            function f() public returns (uint, uint, uint) {
                int a = 43;
                return uint(a);
            }
        }