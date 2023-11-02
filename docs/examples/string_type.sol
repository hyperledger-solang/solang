contract example {
    function test1(string s) public returns (bool) {
        string str = string.concat("Hello, ", s, "!");

        return (str == "Hello, World!");
    }

    function test2(string s, int64 n) public returns (string res) {
        res = "Hello, {}! #{}".format(s, n);
    }
}
