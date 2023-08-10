struct S {
    uint64 f1;
    bool f2;
}

contract foo {
    S[] store;

    function push1(S x) public {
        store.push(x);
    }

    function push2(S x) public {
        S storage f = store.push();
        f.f1 = x.f1;
        f.f2 = x.f2;
    }

    function push_empty() public {
        store.push();
    }

    function pop() public returns (S) {
        return store.pop();
    }

    function len() public returns (uint) {
        return store.length;
    }

    function subscript(uint32 i) public returns (S) {
        return store[i];
    }

    function copy() public returns (S[] memory) {
        return store;
    }

    function set(S[] memory n) public {
        store = n;
    }

    function rm() public {
        delete store;
    }
}
