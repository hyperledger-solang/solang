
        contract c {
            modifier foo bar {}
        }
// ----
// error (47-50): function modifiers or base contracts are not allowed on modifier
// error (53-53): missing '_' in modifier
