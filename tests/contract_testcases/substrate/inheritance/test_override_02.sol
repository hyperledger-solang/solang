
        contract base {
            function foo(uint64 a) override(bar) private returns (uint64) {
                return a + 102;
            }
        }

        contract bar {
            function f() private {}
        }
        
// ---- Expect: diagnostics ----
// error: 3:13-74: 'foo' does not override anything
// error: 3:45-48: override 'bar' is not a base contract of 'base'
