
        contract foo {
            enum foo2 { bar1, bar2 }
            function bar() public returns (int[10] storage x) {
            }
        }
// ---- Expect: diagnostics ----
// error: 4:52-59: return type of type 'storage' not allowed public or external functions
