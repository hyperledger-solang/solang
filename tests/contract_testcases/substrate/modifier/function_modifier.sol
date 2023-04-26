
        contract c {
            modifier foo() { _; }

            function bar() foo2 public {}
        }
// ----
// error (84-88): unknown modifier 'foo2'
