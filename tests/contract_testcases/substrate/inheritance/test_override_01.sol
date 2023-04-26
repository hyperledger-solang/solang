
        contract base {
            function foo(uint64 a) override(bar) private returns (uint64) {
                return a + 102;
            }
        }
        
// ----
// error (37-98): 'foo' does not override anything
// error (69-72): 'bar' not found
