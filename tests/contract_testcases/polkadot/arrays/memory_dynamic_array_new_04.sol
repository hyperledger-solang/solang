
        contract foo {
            function test() public {
                int32[] memory a = new bool(1);

                assert(a.length == 5);
            }
        }
// ---- Expect: diagnostics ----
// error: 4:36-47: new cannot allocate type 'bool'
