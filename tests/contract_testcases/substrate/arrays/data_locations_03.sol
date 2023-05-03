
        contract foo {
            enum foo2 { bar1, bar2 }
            function bar(foo2 x) public returns (uint calldata) {
            }
        }
// ---- Expect: diagnostics ----
// error: 4:55-63: data location 'calldata' can only be specified for array, struct or mapping
