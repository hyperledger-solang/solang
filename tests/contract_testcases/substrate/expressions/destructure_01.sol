contract test {
            function foo(uint bar) public {
                int a;
                int b;

                (c, b) = (1, 2);
            }
        }
// ----
// error (124-125): 'c' not found
