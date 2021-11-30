
        abstract contract y {
            function foo() internal virtual returns (int);
        }

        contract x is y {
            function foo() private override returns (int) {
                return 102;
            }
        }
        