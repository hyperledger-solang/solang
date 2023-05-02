
        contract c {
            function test() public {
                try new other() returns (int32, int[2] storage) {
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
// error: 4:21-32: constructor returns single contract, not 2 values
