
        contract c {
            modifier foo() payable {}
        }
// ---- Expect: diagnostics ----
// error: 3:28-35: modifier cannot have mutability specifier
