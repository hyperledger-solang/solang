
        contract c {
            modifier foo() public {}
        }
// ---- Expect: diagnostics ----
// error: 3:28-34: 'public': modifiers can not have visibility
// error: 3:37: missing '_' in modifier
