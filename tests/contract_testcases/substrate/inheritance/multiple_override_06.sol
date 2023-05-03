
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
        
// ---- Expect: diagnostics ----
// error: 7:13-80: function 'foo' overrides function in same contract
// 	note 3:13-80: previous definition of 'foo'
