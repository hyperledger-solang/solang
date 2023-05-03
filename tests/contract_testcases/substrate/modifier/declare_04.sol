
        contract c {
            modifier foo bar {}
        }
// ---- Expect: diagnostics ----
// error: 3:26-29: function modifiers or base contracts are not allowed on modifier
// error: 3:32: missing '_' in modifier
