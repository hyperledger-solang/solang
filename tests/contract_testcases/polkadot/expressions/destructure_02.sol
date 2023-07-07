contract test {
            function foo(uint bar) public {
                int a;
                int b;

                (a, b) = (1, 2);
            }
        }

// ---- Expect: diagnostics ----
// warning: 2:13-42: function can be declared 'pure'
// warning: 2:31-34: function parameter 'bar' is unused
// warning: 3:21-22: local variable 'a' has been assigned, but never read
// warning: 4:21-22: local variable 'b' has been assigned, but never read
