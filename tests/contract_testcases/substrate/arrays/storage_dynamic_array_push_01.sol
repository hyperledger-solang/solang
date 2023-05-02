
        contract foo {
            int32[4] bar;

            function test() public {
                bar.push(102);
            }
        }
// ---- Expect: diagnostics ----
// error: 6:21-25: method 'push()' not allowed on fixed length array
