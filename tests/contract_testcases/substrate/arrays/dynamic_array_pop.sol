
        contract foo {
            function test() public {
                int[] bar = new int[](1);
                bar.pop(102);
            }
        }
// ---- Expect: diagnostics ----
// error: 5:21-24: method 'pop()' does not take any arguments
