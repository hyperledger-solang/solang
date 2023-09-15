// RUN: --target solana --emit cfg

import 'solana';

contract creator {
    Child public c;
    // BEGIN-CHECK: creator::creator::function::create_child_with_meta__address_address
    function create_child_with_meta(address child, address payer) external {
        AccountMeta[2] metas = [
            AccountMeta({pubkey: child, is_signer: false, is_writable: false}),
            AccountMeta({pubkey: payer, is_signer: true, is_writable: true})
        ];
        // CHECK: constructor(no: 4) salt: value: gas:uint64 0 address:address 0xadde28d6c5697771bb24a668136224c7aac8e8ba974c2881484973b2e762fb74 seeds: Child encoded buffer: %abi_encoded.temp.15 accounts: %metas
        c = new Child{accounts: metas}();

        c.say_hello();
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