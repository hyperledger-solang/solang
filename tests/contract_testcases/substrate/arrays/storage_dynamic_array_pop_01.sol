
        contract foo {
            int32[4] bar;

            function test() public {
                bar.pop();
            }
        }
// ----
// error (108-111): method 'pop()' not allowed on fixed length array
