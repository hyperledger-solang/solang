// RUN: --target solana --emit cfg

import 'solana';

contract creator {
    // BEGIN-CHECK: creator::creator::function::create_child_with_meta__address_address
    function create_child_with_meta(address child, address payer) external {
        AccountMeta[2] metas = [
            AccountMeta({pubkey: child, is_signer: false, is_writable: false}),
            AccountMeta({pubkey: payer, is_signer: true, is_writable: true})
        ];
        // CHECK: external call::regular address:address 0xadde28d6c5697771bb24a668136224c7aac8e8ba974c2881484973b2e762fb74 payload:%abi_encoded.temp.13 value:uint64 0 gas:uint64 0 accounts:%metas seeds: contract|function:(1, 3) flags:
        Child.new{accounts: metas}();

        Child.say_hello();
    }
}

@program_id("Chi1d5XD6nTAp2EyaNGqMxZzUjh6NvhXRxbGHP3D1RaT")
contract Child {
    @payer(payer)
    @space(511 + 7)
    constructor() {
        print("In child constructor");
    }

    function say_hello() pure public {
        print("Hello there");
    }
}