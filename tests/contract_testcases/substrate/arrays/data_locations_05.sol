
        contract foo {
            enum foo2 { bar1, bar2 }
            function bar(foo2 x) public returns (int storage) {
            }
        }
// ----
// error (114-121): data location 'storage' can only be specified for array, struct or mapping
