
        contract foo {
            enum foo2 { bar1, bar2 }
            function bar(foo2 memory x) public returns () {
            }
        }
// ----
// error (91-97): data location 'memory' can only be specified for array, struct or mapping
