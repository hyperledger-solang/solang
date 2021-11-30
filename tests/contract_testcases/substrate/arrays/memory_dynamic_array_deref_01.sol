
        contract foo {
            function test() public {
                int32[] memory a = new int32[](2);
                int32 i = 1;

                a[i] = 5;
            }
        }