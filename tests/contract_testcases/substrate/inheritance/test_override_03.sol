
        contract base {
            function foo(uint64 a) override private returns (uint64) {
                return a + 102;
            }
        }
        
// ----
// error (37-93): 'foo' does not override anything
