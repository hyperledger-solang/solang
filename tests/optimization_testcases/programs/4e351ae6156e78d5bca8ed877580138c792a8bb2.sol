contract foo {
    mapping(string => mapping(int64 => bytes1)) public map;

    function set(string s, int64 n, bytes1 v) public {
        map[s][n] = v;
    }
}
