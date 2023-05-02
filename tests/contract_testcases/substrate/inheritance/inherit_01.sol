        contract a is foo {
            constructor(int arg1) public {
            }
        }

        contract b is bar.foo {
            constructor(int arg1) public {
            }
        }

// ---- Expect: diagnostics ----
// error: 1:23-26: 'foo' not found
// warning: 2:35-41: 'public': visibility for constructors is ignored
// error: 6:23-26: 'bar' not found
// warning: 7:35-41: 'public': visibility for constructors is ignored
