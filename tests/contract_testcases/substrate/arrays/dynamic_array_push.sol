
        contract foo {
            function test() public {
                int[] bar = new int[](2);
                assert(bar.length == 0);
                bar.push(102, 20);
            }
        }
// ---- Expect: diagnostics ----
// error: 6:21-25: method 'push()' takes at most 1 argument
