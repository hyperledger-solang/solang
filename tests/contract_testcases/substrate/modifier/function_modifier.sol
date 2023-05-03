
        contract c {
            modifier foo() { _; }

            function bar() foo2 public {}
        }
// ---- Expect: diagnostics ----
// error: 5:28-32: unknown modifier 'foo2'
