contract foo {
    string s;

    function set(string value) public {
        s = value;
    }

    function get() public returns (string) {
        return s;
    }
}
