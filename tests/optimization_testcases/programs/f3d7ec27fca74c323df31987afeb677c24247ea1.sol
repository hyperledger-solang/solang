struct S {
    string f1;
    int64[] f2;
}

contract foo {
    S[1e24] map;

    function set_string(uint index, string s) public {
        map[index].f1 = s;
    }

    function add_int(uint index, int64 n) public {
        map[index].f2.push(n);
    }

    function get(uint index) public returns (S) {
        return map[index];
    }

    function rm(uint index) public {
        delete map[index];
    }
}
