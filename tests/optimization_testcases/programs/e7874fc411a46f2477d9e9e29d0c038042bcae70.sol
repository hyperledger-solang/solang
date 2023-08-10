contract foo {
    int64[] store;

    function push(int64 x) public {
        store.push(x);
    }

    function push_zero() public {
        store.push();
    }

    function pop() public returns (int64) {
        return store.pop();
    }

    function len() public returns (uint) {
        return store.length;
    }

    function subscript(uint32 i) public returns (int64) {
        return store[i];
    }

    function copy() public returns (int64[] memory) {
        return store;
    }

    function set(int64[] n) public {
        store = n;
    }

    function rm() public {
        delete store;
    }
}
