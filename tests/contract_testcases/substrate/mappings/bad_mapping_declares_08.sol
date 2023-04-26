
        contract c {
            function test() public {
                int[] x = new mapping(int => address)(2);
                //
            }
        }
// ----
// error (85-115): new cannot allocate type 'mapping(int256 => address)'
