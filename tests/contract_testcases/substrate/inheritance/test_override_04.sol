
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
        
// ----
// error (44-105): function 'foo' overrides function which is not virtual
// 	note (200-247): previous definition of function 'foo'
