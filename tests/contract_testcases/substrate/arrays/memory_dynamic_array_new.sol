
        contract foo {
            function test() public {
                int32[] memory a = new int32[]();

                assert(a.length == 5);
            }
        }
// ---- Expect: diagnostics ----
// error: 4:36-49: new dynamic array should have a single length argument
