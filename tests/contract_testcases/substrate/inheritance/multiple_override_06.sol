
        contract base is bar, bar2 {
            function foo(uint64 a) override(bar,bar2) internal returns (uint64) {
                return a + 104;
            }

            function foo(uint64 a) override(bar,bar2) internal returns (uint64) {
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
        
// ----
// error (179-246): function 'foo' overrides function in same contract
// 	note (50-117): previous definition of 'foo'
