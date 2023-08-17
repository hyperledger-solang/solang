@program_id("Foo5mMfYo5RhRcWa4NZ2bwFn4Kdhe8rNK5jchxsKrivA")
contract Foo {
    function say_hello() public pure {
        print("Hello from foo");
    }
}

contract Bar {
    Foo public foo;

    function create_foo() external {
        foo = new Foo();
    }

    function call_foo() public {
        foo.say_hello();
    }
}
