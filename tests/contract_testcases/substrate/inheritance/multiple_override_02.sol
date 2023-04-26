
        contract base is bar, bar2, bar3 {
            function foo(uint64 a) override(bar,bar2,bar3) internal returns (uint64) {
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

        contract bar3 {
            function f() public {

            }
        }
        
// ----
// error (79-102): function 'foo' includes extraneous overrides 'bar3', specify 'override(bar2,bar)'
