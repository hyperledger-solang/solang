
        abstract contract foo {
            enum e { e1, e2, e3 }
            e[1 / 0] x;
        }
// ---- Expect: diagnostics ----
// error: 4:15-20: divide by zero
