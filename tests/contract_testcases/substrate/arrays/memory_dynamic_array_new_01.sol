
        contract foo {
            function test() public {
                int32[] memory a = new int32[](1, 2);

                assert(a.length == 5);
            }
        }