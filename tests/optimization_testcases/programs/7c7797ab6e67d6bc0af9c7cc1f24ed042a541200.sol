contract foo {
    int private foo;

    function boom() public view returns (int) {
        int baz = false ? foo : 0;
        return baz;
    }
}
