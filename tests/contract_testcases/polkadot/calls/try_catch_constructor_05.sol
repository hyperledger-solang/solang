
        contract c {
            function test() public {
                try new other() {
                    x = 1;
                } {
                    x= 5;
                }
                catch (string) {
                    x = 2;
                }
                assert(x == 1);
            }
        }

        contract other {
            function test() public  {
            }
        }
        
// ---- Expect: diagnostics ----
// error: 4:33-6:18: unexpected code block
