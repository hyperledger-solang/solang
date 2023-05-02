
        contract base is bar, bar2 {
            function foo(uint64 a) virtual internal returns (uint64) {
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
// error: 3:13-69: function 'foo' should specify override list 'override(bar2,bar)'
