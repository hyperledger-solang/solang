// RUN: --target solana --emit cfg
contract Foo {
    uint b;
    function get_b() public returns (uint) {
        return b;
    }
}

contract Other {
    // BEGIN-CHECK: Other::Other::function::call_foo__address
    function call_foo(address id) external {
        // The account must be properly indexed so that the call works.
        // CHECK: constructor(no: ) salt: value: gas:uint64 0 address:(arg #0) seeds: Foo encoded buffer: %abi_encoded.temp.12 accounts: [1] [ struct { (load (struct (subscript struct AccountInfo[] (builtin Accounts ())[uint32 0]) field 0)), true, false } ]
        Foo.new{program_id: id}();
    }
}