contract foo {
    string[4] store;

    function set_elem(uint index, string val) public {
        store[index] = val;
    }

    function get_elem(uint index) public returns (string) {
        return store[index];
    }

    function set(string[4] x) public {
        store = x;
    }

    function get() public returns (string[4]) {
        return store;
    }

    function del() public {
        delete store;
    }
}
