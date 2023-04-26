
        library c {
            function foo() public { }
        }

        contract a is c {
            function bar() public { }
        }
// ----
// error (92-93): library 'c' cannot be used as base contract for contract 'a'
