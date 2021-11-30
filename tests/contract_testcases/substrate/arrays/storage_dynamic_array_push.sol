
        contract foo {
            int32[] bar;

            function test() public {
                assert(bar.length == 0);
                bar.push(102, 20);
            }
        }