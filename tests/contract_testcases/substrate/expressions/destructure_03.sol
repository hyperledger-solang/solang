contract test {
            function foo(uint bar) public {
                int a;
                int b;

                (a , b) = (1, );
            }
        }
// ----
// error (137-137): stray comma
