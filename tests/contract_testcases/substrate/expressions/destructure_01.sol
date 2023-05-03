contract test {
            function foo(uint bar) public {
                int a;
                int b;

                (c, b) = (1, 2);
            }
        }
// ---- Expect: diagnostics ----
// error: 6:18-19: 'c' not found
