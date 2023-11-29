

event Foo(int no, string name);

contract c {
    function foo() public {
        emit Foo(a, b);
        emit Foo({ no: a, name: b, no: c});
        emit Foo2({ no: a, name: b, no: c});
        emit Foo({ no: 1, name: "name", foo: 5});
    }
}

// ---- Expect: diagnostics ----
// error: 7:18-19: 'a' not found
// error: 7:21-22: 'b' not found
// error: 8:24-25: 'a' not found
// error: 8:33-34: 'b' not found
// error: 8:36-38: duplicate argument with name 'no'
// error: 8:40-41: 'c' is a contract
// error: 9:14-18: event 'Foo2' not found
// error: 9:25-26: 'a' not found
// error: 9:34-35: 'b' not found
// error: 9:37-39: duplicate argument with name 'no'
// error: 9:41-42: 'c' is a contract
// error: 10:9-49: event expects 2 arguments, 3 provided
// 	note 3:7-10: definition of Foo
