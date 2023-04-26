contract test {
            function foo(uint bar) public {
                int a;
                int b;

                (a, b) = (1, 2);
            }
        }

// ----
// warning (28-57): function can be declared 'pure'
// warning (46-49): function parameter 'bar' has never been read
// warning (80-81): local variable 'a' has been assigned, but never read
// warning (103-104): local variable 'b' has been assigned, but never read
