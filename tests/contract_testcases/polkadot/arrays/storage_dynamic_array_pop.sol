
        contract foo {
            int32[] bar;

            function test() public {
                assert(bar.length == 0);
                bar.pop(102);
            }
        }
// ---- Expect: diagnostics ----
// error: 7:21-24: method 'pop()' does not take any arguments
