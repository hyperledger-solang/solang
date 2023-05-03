
        abstract contract foo {
            function bar(uint storage) public returns () {
            }
        }
// ---- Expect: diagnostics ----
// error: 3:31-38: data location 'storage' can only be specified for array, struct or mapping
