
        contract c {
            modifier foo() pure {}
        }
// ---- Expect: diagnostics ----
// error: 3:28-32: modifier cannot have mutability specifier
