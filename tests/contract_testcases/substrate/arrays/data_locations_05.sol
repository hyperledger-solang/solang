
        contract foo {
            enum foo2 { bar1, bar2 }
            function bar(foo2 x) public returns (int storage) {
            }
        }
// ---- Expect: diagnostics ----
// error: 4:54-61: data location 'storage' can only be specified for array, struct or mapping
