
        contract c {
            modifier foo() {
                _;
            }

            function bar() public {
                foo();
            }
        }