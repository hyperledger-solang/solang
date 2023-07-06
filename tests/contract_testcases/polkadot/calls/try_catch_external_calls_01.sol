
        contract c {
            function test() public {
                other o = new other();
                int32 x = 0;
                try o.test() returns (int32, int[2] storage) {
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
// error: 6:46-52: type 'int256[2] storage' does not match return value of function 'bool'
