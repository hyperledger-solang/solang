
        contract base is bar, bar2 {
            function foo(uint64 a) override(bar,bar2) internal returns (uint64) {
                return a + 102;
            }
        }

        contract bar {
            function foo(uint64 a) internal returns (uint64) {
                return a + 102;
            }
        }

        contract bar2 {
            function foo(uint64 a) virtual internal returns (uint64) {
                return a + 103;
            }
        }
        
// ---- Expect: diagnostics ----
// error: 3:13-80: function 'foo' overrides functions which are not 'virtual'
// 	note 9:13-61: function 'foo' is not specified 'virtual'
