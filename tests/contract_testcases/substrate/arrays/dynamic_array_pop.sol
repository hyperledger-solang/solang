
        contract foo {
            function test() public {
                int[] bar = new int[](1);
                bar.pop(102);
            }
        }