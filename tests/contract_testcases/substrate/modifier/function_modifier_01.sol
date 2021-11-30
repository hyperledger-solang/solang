
        contract c {
            modifier foo() { _; }

            function bar() foo(1) public {}
        }