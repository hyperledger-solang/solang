contract foo {
    int64 constant foo = 1;
    int64 bar = 2;

    function list() public returns (int64[3]) {
        return [foo, bar, 3];
    }
}
