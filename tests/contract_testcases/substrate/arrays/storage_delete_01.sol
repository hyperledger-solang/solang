
        contract foo {
            int32[] bar;

            function test() public {
                int32 x = delete bar;
            }
        }