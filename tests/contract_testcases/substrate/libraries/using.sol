
        contract c {
            using x for x;
        }
// ---- Expect: diagnostics ----
// error: 3:25-26: type 'x' not found
