
        contract c {
            modifier foo() {
                _;
            }

            function bar() public {
                foo();
            }
        }
// ----
// error (137-140): unknown function or type 'foo'
