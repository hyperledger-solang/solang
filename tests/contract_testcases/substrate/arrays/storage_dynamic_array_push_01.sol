
        contract foo {
            int32[4] bar;

            function test() public {
                bar.push(102);
            }
        }
// ----
// error (108-112): method 'push()' not allowed on fixed length array
