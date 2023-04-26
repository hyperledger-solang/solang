
        contract foo {
            function test() public {
                int32[] memory a = new int32[](hex"ab");

                assert(a.length == 5);
            }
        }
// ----
// error (108-115): new dynamic array should have an unsigned length argument
