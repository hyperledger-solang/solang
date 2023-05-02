
        contract foo {
            function test() public {
                int32[] memory a = new int32[](hex"ab");

                assert(a.length == 5);
            }
        }
// ---- Expect: diagnostics ----
// error: 4:48-55: new dynamic array should have an unsigned length argument
