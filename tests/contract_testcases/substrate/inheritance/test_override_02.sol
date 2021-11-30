
        contract base {
            function foo(uint64 a) override(bar) private returns (uint64) {
                return a + 102;
            }
        }

        contract bar {
            function f() private {}
        }
        