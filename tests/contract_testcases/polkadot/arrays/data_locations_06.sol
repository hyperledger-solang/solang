
        contract foo {
            enum foo2 { bar1, bar2 }
            function bar(int[10] storage x) public returns (int) {
            }
        }
// ---- Expect: diagnostics ----
// error: 4:34-41: parameter of type 'storage' not allowed public or external functions
