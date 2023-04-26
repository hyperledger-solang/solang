
        contract foo {
            function test() public {
                int32[] memory a = new int32[]();

                assert(a.length == 5);
            }
        }
// ----
// error (96-109): new dynamic array should have a single length argument
