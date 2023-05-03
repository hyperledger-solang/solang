
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
        
// ---- Expect: diagnostics ----
// error: 3:36-50: function 'foo' override list does not contain 'bar'
// 	note 9:13-69: previous definition of function 'foo'
