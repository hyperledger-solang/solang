
        abstract contract y {
            function foo() external view virtual returns (int);
        }

        contract x is y {
            function foo() external payable override returns (int) {
                return 102;
            }
        }
        