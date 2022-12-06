@program_id("Foo5mMfYo5RhRcWa4NZ2bwFn4Kdhe8rNK5jchxsKrivA")
contract Foo {
    function say_hello() public pure {
        print("Hello from foo");
    }
}

contract Bar {
    Foo public foo;

    function create_foo(address new_address) public {
        foo = new Foo{address: new_address}();
    }

    function call_foo() public pure {
        foo.say_hello();
    }
}
