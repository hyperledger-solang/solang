
        contract c {
            modifier foo() internal {}
        }
// ---- Expect: diagnostics ----
// error: 3:28-36: 'internal': modifiers can not have visibility
// error: 3:39: missing '_' in modifier
