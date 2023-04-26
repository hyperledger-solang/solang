
        contract base {
            function foo(uint64 a) override(bar) private returns (uint64) {
                return a + 102;
            }
        }

        contract bar {
            function f() private {}
        }
        
// ----
// error (37-98): 'foo' does not override anything
// error (69-72): override 'bar' is not a base contract of 'base'
