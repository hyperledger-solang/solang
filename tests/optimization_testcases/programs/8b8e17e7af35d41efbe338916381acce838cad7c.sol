contract foo {
    function f() public returns (string) {
        (string a, string b) = ("Hello", "World!");
        return (a);
    }
}
