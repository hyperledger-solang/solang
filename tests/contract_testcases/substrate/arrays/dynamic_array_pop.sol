
        contract foo {
            function test() public {
                int[] bar = new int[](1);
                bar.pop(102);
            }
        }
// ----
// error (123-126): method 'pop()' does not take any arguments
