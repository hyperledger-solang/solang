
        contract c {
            modifier foo() {
                _;
            }

            function bar() public {
                foo();
            }
        }
// ---- Expect: diagnostics ----
// error: 8:17-20: unknown function or type 'foo'
