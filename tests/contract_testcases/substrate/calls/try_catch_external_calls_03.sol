
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
        
// ----
// error (171-172): x is already declared
// 	note (120-121): location of previous declaration
// error (234-240): catch can only take 'bytes memory', not 'string'
