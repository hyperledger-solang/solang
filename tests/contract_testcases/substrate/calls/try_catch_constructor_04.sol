
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
        
// ----
// error (79-79): code block missing for no catch
