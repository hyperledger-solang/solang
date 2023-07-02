
        contract c {
            function test() public {
                other o = new other();
                int32 x = 0;
                try o.test() returns (int32, bool) {
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
// error: 8:26-32: catch can only take 'bytes memory', not 'string'
