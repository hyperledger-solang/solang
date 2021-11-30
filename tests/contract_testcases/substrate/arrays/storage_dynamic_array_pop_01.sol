
        contract foo {
            int32[4] bar;

            function test() public {
                bar.pop();
            }
        }