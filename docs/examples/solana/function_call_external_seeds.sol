import 'solana';

contract c {
    address constant token = address"mv3ekLzLbnVPNxjSKvqBpU3ZeZXPQdEC3bp5MDEBG68";

    function test(address addr, address addr2, bytes seed) public {
        bytes instr = new bytes(1);

        instr[0] = 1;

        AccountMeta[2] metas = [
            AccountMeta({pubkey: addr, is_writable: true, is_signer: true}),
            AccountMeta({pubkey: addr2, is_writable: true, is_signer: true})
        ];

        token.call{accounts: metas, seeds: [ [ "test", seed ], [ "foo", "bar "] ]}(instr);
    }
}
