@program_id("Foo5mMfYo5RhRcWa4NZ2bwFn4Kdhe8rNK5jchxsKrivA")
contract Foo {
    function say_hello() public pure {
        print("Hello from foo");
    }
}

contract OtherFoo {
    function say_bye() public pure {
        print("Bye from other foo");
    }
}

contract Bar {
    function create_foo() external {
        Foo.new();
    }

    function call_foo() public {
        Foo.say_hello{accounts: []}();
    }

    function foo_at_another_address(address other_foo_id) external {
        OtherFoo.new{program_id: other_foo_id}();
        OtherFoo.say_bye{program_id: other_foo_id}();
    }
}
