
        contract c {
            function test() public {
                other o = new other();
                int32 x = 0;
                try o.test() returns (int32 x, bool) {
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
// error: 6:45-46: x is already declared
// 	note 5:23-24: location of previous declaration
// error: 8:26-32: catch can only take 'bytes memory', not 'string'
