// RUN: --target solana --emit cfg

import 'solana';

@program_id("6qEm4QUJGFvqKNJGjTrAEiFhbVBY4ashpBjDHEFvEUmW")
contract Foo {
    uint b;
    constructor(uint a) {
        b = a;
    }

    function get_b(address id) public returns (uint) {
        return b;
    }

    function get_b2(address id) public returns (uint) {
        return b;
    }
}

@program_id("9VLAw4to9KsX9DvyzJUJwfUeQCouX79szENj78sZKiqA")
contract Other is Foo {
    uint c;
    constructor(uint d) Foo(d) {
        c = d;
    }
    // BEGIN-CHECK: Other::Other::function::call_foo__address
    function call_foo(address id) external {
        // internal calls
        Foo.get_b(id);
        // CHECK: call Other::Foo::function::get_b__address (arg #0)
        Foo.get_b2({id: id});
        // CHECK: call Other::Foo::function::get_b2__address (arg #0)
    }
    // BEGIN-CHECK: Other::Other::function::call_foo2__address_address
    function call_foo2(address id, address acc) external {
        AccountMeta[1] meta = [
            AccountMeta({pubkey: acc, is_writable: false, is_signer: false})
        ];
        // external calls
        Foo.get_b{program_id: id, accounts: meta}(id);
        // CHECK: external call::regular address:(arg #0) payload:%abi_encoded.temp.35 value:uint64 0 gas:uint64 0 accounts:%meta seeds: contract|function:(0, 3) flags:
        Foo.get_b2{program_id: id, accounts: meta}(id);
        // CHECK: external call::regular address:(arg #0) payload:%abi_encoded.temp.36 value:uint64 0 gas:uint64 0 accounts:%meta seeds: contract|function:(0, 4) flags:
    }
}