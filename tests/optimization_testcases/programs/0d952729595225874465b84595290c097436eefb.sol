contract foo {
    int64[4] store;

    function set_elem(uint index, int64 val) public {
        store[index] = val;
    }

    function get_elem(uint index) public returns (int64) {
        return store[index];
    }

    function set(int64[4] x) public {
        store = x;
    }

    function get() public returns (int64[4]) {
        return store;
    }

    function del() public {
        delete store;
    }
}
