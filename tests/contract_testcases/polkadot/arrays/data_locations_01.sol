
        contract foo {
            function bar(uint calldata x) public returns () {
            }
        }
// ---- Expect: diagnostics ----
// error: 3:31-39: data location 'calldata' can only be specified for array, struct or mapping
