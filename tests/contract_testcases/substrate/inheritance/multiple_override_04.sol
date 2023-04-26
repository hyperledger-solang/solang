
        contract base is bar, bar2 {
            function foo(uint64 a) internal returns (uint64) {
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
        
// ----
// error (50-98): function 'foo' should specify override list 'override(bar2,bar)'
// error (345-401): function 'foo' with this signature already defined
// 	note (193-249): previous definition of function 'foo'
