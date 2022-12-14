contract foo {
    // The selector attribute can be an array of values (bytes)
    @selector([1, 2, 3, 4])
    function get_foo() pure public returns (int) {
        return 102;
    }

    @selector([0x05, 0x06, 0x07, 0x08])
    function get_bar() pure public returns (int) {
        return 105;
    }
}
