
        contract foo {
            enum foo2 { bar1, bar2 }
            function bar() public returns (int[10] storage x) {
            }
        }
// ----
// error (112-119): return type of type 'storage' not allowed public or external functions
