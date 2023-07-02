
        contract c {
            function test() public {
                try new other() returns (int32) {
                    x = 1;
                } catch (string) {
                    x = 2;
                }
                assert(x == 1);
            }
        }

        contract other {
            function test() public returns (int32, bool) {
                return (102, true);
            }
        }
        
// ---- Expect: diagnostics ----
// error: 4:42-47: type 'int32' does not match return value of function 'contract other'
