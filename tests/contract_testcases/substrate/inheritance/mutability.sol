
        contract y {
            function foo() external pure virtual returns (int) {
                return 102;
            }
        }

        contract x is y {
            function foo() external override returns (int) {
                return 102;
            }
        }
        