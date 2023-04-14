// RUN: --target solana --emit cfg

import 'solana';

contract creator {
    Child public c;
    // BEGIN-CHECK: creator::creator::function::create_child_with_meta__address_address
    function create_child_with_meta(address child, address payer) public {
        AccountMeta[2] metas = [
            AccountMeta({pubkey: child, is_signer: false, is_writable: false}),
            AccountMeta({pubkey: payer, is_signer: true, is_writable: true})
        ];
        // CHECK: constructor salt: value: gas:uint64 0 address: seeds: Child encoded buffer: %abi_encoded.temp.16 accounts: %metas
        c = new Child{accounts: metas}(payer);

        c.say_hello();
    }
}

@program_id("Chi1d5XD6nTAp2EyaNGqMxZzUjh6NvhXRxbGHP3D1RaT")
contract Child {
    @payer(payer)
    @space(511 + 7)
    constructor(address payer) {
        print("In child constructor");
    }

    function say_hello() pure public {
        print("Hello there");
    }
}