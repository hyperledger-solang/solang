
        contract c {
            modifier foo() public {}
        }
// ----
// error (49-55): 'public': modifiers can not have visibility
// error (58-58): missing '_' in modifier
