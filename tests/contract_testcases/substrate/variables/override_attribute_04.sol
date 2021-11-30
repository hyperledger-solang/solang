contract x is y {
            int public foo;
        }

        contract y {
            function foo() public virtual returns (int) {
                return 102;
            }
        }
        