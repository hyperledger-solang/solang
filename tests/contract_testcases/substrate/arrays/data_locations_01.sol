
        contract foo {
            function bar(uint calldata x) public returns () {
            }
        }
// ----
// error (54-62): data location 'calldata' can only be specified for array, struct or mapping
