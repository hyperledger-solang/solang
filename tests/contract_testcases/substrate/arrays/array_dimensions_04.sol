
        abstract contract foo {
            struct bar {
                int32 x;
            }
            bar[1 % 0] x;
        }
// ---- Expect: diagnostics ----
// error: 6:17-22: divide by zero
