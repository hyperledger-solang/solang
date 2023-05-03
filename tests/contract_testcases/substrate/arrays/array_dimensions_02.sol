
        abstract contract foo {
            bool[1 / 10] x;
        }
// ---- Expect: diagnostics ----
// error: 3:18-24: zero size array not permitted
