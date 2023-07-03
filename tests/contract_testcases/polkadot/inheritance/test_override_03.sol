
        contract base {
            function foo(uint64 a) override private returns (uint64) {
                return a + 102;
            }
        }
        
// ---- Expect: diagnostics ----
// error: 3:13-69: 'foo' does not override anything
