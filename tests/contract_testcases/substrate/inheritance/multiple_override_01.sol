
        contract base is bar, bar2 {
            function foo(uint64 a) override(bar) internal returns (uint64) {
                return a + 102;
            }
        }

        contract bar {
            function foo(uint64 a) virtual internal returns (uint64) {
                return a + 102;
            }
        }

        contract bar2 {
            function foo(uint64 a) virtual internal returns (uint64) {
                return a + 103;
            }
        }
        