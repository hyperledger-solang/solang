
        library c {
            function foo() public { }
        }

        contract a is c {
            function bar() public { }
        }
// ---- Expect: diagnostics ----
// error: 6:23-24: library 'c' cannot be used as base contract for contract 'a'
