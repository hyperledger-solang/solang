
        contract foo {
            struct s {
                int32 f1;
                bool f2;
            }
            s[] bar;

            function test() public {
                s storage n = bar.push(s(-1, false));
            }
        }
// ---- Expect: diagnostics ----
// error: 10:35-39: function or method does not return a value
