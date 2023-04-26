
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
        
// ----
// error (73-81): function 'foo' should specify override list 'override(bar2,bar)'
