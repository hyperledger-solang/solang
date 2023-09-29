contract Foo {
    uint b;
    function get_b(address id) external returns (uint) {
        Other.new{program_id: id}();
        return b;
    }
}

contract Other {
    function call_foo(address id) external {
        Foo.new{program_id: id}();
    }
}

contract Foo2 {
    uint b;
    constructor(uint a) {
        b = a;
    }

    function get_b(address id) external returns (uint) {
        Other2.new{program_id: id}(2);
        return b;
    }

    function get_b2(address id) external returns (uint) {
        Other2.new{program_id: id}({d: 2});
        return b;
    }
}

contract Other2 {
    uint c;
    constructor(uint d) {
        c = d;
    }
    function call_foo(address id) external {
        Foo2.new{program_id: id}(5);
    }
    function call_foo2(address id) external {
        Foo2.new{program_id: id}({a: 5});
    }
}

// ---- Expect: diagnostics ----
// error: 11:9-34: circular reference creating contract 'Foo'
// error: 38:9-36: circular reference creating contract 'Foo2'
// error: 41:9-41: circular reference creating contract 'Foo2'
