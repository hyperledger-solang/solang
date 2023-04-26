
        contract foo {
            enum foo2 { bar1, bar2 }
            function bar(foo2 x) public returns (uint calldata) {
            }
        }
// ----
// error (115-123): data location 'calldata' can only be specified for array, struct or mapping
