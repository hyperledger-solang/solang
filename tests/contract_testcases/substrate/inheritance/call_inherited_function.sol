
        contract base {
            function foo() private returns (uint64) {
                return 102;
            }
        }

        contract apex is base {
            function bar() public returns (uint64) {
                return foo() + 3;
            }
        }
        