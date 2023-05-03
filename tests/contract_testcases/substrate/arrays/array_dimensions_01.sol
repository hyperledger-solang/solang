
        abstract contract foo {
            bool[-10 + 10] x;
        }
// ---- Expect: diagnostics ----
// error: 3:18-26: zero size array not permitted
