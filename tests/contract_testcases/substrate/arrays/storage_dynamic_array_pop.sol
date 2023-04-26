
        contract foo {
            int32[] bar;

            function test() public {
                assert(bar.length == 0);
                bar.pop(102);
            }
        }
// ----
// error (148-151): method 'pop()' does not take any arguments
