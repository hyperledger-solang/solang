
        contract base is bar, bar2 {
            function foo(uint64 a) override(bar2) internal returns (uint64) {
                return a + 102;
            }
        }

        contract bar {
            function foo(uint64 a) virtual internal returns (uint64) {
                return a + 102;
            }
        }

        contract bar2 {
            uint64 public x;
        }
        