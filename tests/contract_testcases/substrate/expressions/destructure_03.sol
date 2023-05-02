contract test {
            function foo(uint bar) public {
                int a;
                int b;

                (a , b) = (1, );
            }
        }
// ---- Expect: diagnostics ----
// error: 6:31: stray comma
