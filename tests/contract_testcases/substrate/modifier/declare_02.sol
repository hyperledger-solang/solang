
        contract c {
            modifier foo() payable {}
        }
// ----
// error (49-56): modifier cannot have mutability specifier
