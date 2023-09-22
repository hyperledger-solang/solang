abstract contract Foo2 {
    uint b;
    constructor(uint a) {
        b = a;
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
// error: 14:9-36: cannot construct 'Foo2' of type 'abstract contract'
// error: 17:9-41: cannot construct 'Foo2' of type 'abstract contract'
