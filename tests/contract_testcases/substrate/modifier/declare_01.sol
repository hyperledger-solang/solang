
        contract c {
            modifier foo() internal {}
        }
// ----
// error (49-57): 'internal': modifiers can not have visibility
// error (60-60): missing '_' in modifier
