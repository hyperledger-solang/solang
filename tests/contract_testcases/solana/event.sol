

event Foo(int no, string name);

contract c {
    function foo() public {
        emit Foo(a, b);
        emit Foo({ no: a, name: b, no: c});
        emit Foo2({ no: a, name: b, no: c});
    }
}
// ----
// error (93-94): 'a' not found
// error (123-124): 'a' not found
// error (132-133): 'b' not found
// error (135-137): duplicate argument with name 'no'
// error (139-140): 'c' is a contract
// error (168-169): 'a' not found
// error (177-178): 'b' not found
// error (180-182): duplicate argument with name 'no'
// error (184-185): 'c' is a contract
