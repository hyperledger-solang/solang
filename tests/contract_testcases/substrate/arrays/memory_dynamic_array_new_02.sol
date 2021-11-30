
        contract foo {
            function test() public {
                int32[] memory a = new int32[](hex"ab");

                assert(a.length == 5);
            }
        }