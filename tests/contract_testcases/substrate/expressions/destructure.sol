contract test {
            function foo(uint bar) public {
                int a;
                int b;

                (a, b) = (1, 2, 3);
            }
        }
// ---- Expect: diagnostics ----
// error: 6:17-35: destructuring assignment has 2 elements on the left and 3 on the right
