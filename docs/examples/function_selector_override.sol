contract foo {
    function get_foo() selector=hex"01020304" public returns (int) {
        return 102;
    }
}
