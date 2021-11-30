
        contract c {
            uint64 var;
            modifier foo() { uint64 x = var; _; }

            function bar() foo() public pure {}
        }