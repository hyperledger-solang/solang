
        contract base is bar, bar2 {
            function foo(uint64 a) override internal returns (uint64) {
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
// error: 3:36-44: function 'foo' should specify override list 'override(bar2,bar)'
