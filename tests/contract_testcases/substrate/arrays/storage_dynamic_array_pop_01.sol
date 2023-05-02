
        contract foo {
            int32[4] bar;

            function test() public {
                bar.pop();
            }
        }
// ---- Expect: diagnostics ----
// error: 6:21-24: method 'pop()' not allowed on fixed length array
