
        contract foo {
            function test() public {
                int32[] memory a = new bool(1);

                assert(a.length == 5);
            }
        }
// ----
// error (96-107): new cannot allocate type 'bool'
