
        abstract contract c {
            modifier foo() {}
        }
// ---- Expect: diagnostics ----
// error: 3:30: missing '_' in modifier
