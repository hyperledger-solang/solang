
        contract c {
            function test() public {
                try new other()
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
// error: 4:21: code block missing for no catch
