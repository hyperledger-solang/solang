
        contract foo {
            function test() public {
                int32[] memory a = new int32[](2);

                a[-1] = 5;
            }
        }