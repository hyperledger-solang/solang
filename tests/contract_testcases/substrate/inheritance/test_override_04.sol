
        contract base is bar {
            function foo(uint64 a) override(bar) private returns (uint64) {
                return a + 102;
            }
        }

        contract bar {
            function foo(uint64 a) private returns (uint64) {
                return a + 102;
            }
        }
        