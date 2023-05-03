
        contract c {
            function test() public {
                int[] x = new mapping(int => address)(2);
                //
            }
        }
// ---- Expect: diagnostics ----
// error: 4:27-57: new cannot allocate type 'mapping(int256 => address)'
