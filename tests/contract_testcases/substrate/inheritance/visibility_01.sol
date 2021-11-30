
        abstract contract y {
            function foo() external virtual returns (int);
        }

        contract x is y {
            function foo() internal override returns (int) {
                return 102;
            }
        }
        