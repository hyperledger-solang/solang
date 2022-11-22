contract Foo {
    function foo(uint32 n) public {
        if (n > 10) {
            // do something
        }

        // ERROR: unlike C integers can not be used as a condition
        // if (n) {
        //     // ...
        // }
    }
}
