
        abstract contract foo {
            function bar(uint storage) public returns () {
            }
        }
// ----
// error (63-70): data location 'storage' can only be specified for array, struct or mapping
