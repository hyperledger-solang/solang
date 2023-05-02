
        contract base is bar {
            function foo(uint64 a) override(bar) private returns (uint64) {
                return a + 102;
            }
        }

        contract bar {
            function foo(uint64 a) private returns (uint64) {
                return a + 102;
            }
        }
        
// ---- Expect: diagnostics ----
// error: 3:13-74: function 'foo' overrides function which is not virtual
// 	note 9:13-60: previous definition of function 'foo'
