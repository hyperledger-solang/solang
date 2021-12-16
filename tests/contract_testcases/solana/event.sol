

event Foo(int no, string name);

contract c {
    function foo() public {
        emit Foo(a, b);
        emit Foo({ no: a, name: b, no: c});
        emit Foo2({ no: a, name: b, no: c});
    }
}