        contract a is foo {
            constructor(int arg1) public {
            }
        }

        contract b is bar.foo {
            constructor(int arg1) public {
            }
        }

// ----
// error (22-25): 'foo' not found
// warning (62-68): 'public': visibility for constructors is ignored
// error (118-121): 'bar' not found
// warning (162-168): 'public': visibility for constructors is ignored
