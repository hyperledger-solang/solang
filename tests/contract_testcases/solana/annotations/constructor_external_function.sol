@program_id("Foo5mMfYo5RhRcWa4NZ2bwFn4Kdhe8rNK5jchxsKrivA")
contract Foo {
    function say_hello() public pure {
        print("Hello from foo");
    }
}

contract Bar {
    Foo public foo;

    function external_create_foo(address addr) external {
        // This is allowed
        foo = new Foo{address: addr}();
    }

    function create_foo(address new_address) public {
        // This is not allowed
        foo = new Foo{address: new_address}();
    }

    function call_foo() public pure {
        foo.say_hello();
    }
}

// ---- Expect: diagnostics ----
// error: 18:15-46: accounts are required for calling a contract. You can either provide the accounts with the {accounts: ...} call argument or change this function's visibility to external