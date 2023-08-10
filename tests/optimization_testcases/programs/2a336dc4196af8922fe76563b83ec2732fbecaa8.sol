contract foo {
    int64[] store;

    function pop() public returns (int64) {
        return store.pop();
    }
}
