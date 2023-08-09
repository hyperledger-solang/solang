struct S {
    string f1;
    int64[] f2;
}

contract foo {
    mapping(string => S) map;

    function set_string(string index, string s) public {
        map[index].f1 = s;
    }

    function add_int(string index, int64 n) public {
        map[index].f2.push(n);
    }

    function get(string index) public returns (S) {
        return map[index];
    }

    function rm(string index) public {
        delete map[index];
    }
}
