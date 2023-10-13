@program_id("Foo5mMfYo5RhRcWa4NZ2bwFn4Kdhe8rNK5jchxsKrivA")
contract Foo {
    function say_hello() public pure {
        print("Hello from foo");
    }
}

contract Bar {
    function external_create_foo(address addr) external {
        // This not is allowed
        Foo.new{address: addr}();
    }

    function create_foo() public {
        // This is not allowed
        Foo.new();
    }

    function this_is_allowed() external {
        Foo.new();
    }

    function call_foo() public pure {
        Foo.say_hello();
    }
}

// ---- Expect: diagnostics ----
// error: 11:17-30: 'address' not a valid call parameter
// error: 16:9-18: accounts are required for calling a contract. You can either provide the accounts with the {accounts: ...} call argument or change this function's visibility to external
// error: 24:9-24: accounts are required for calling a contract. You can either provide the accounts with the {accounts: ...} call argument or change this function's visibility to external
