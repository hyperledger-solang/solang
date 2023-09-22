import 'solana';

contract creator {
    function create_with_metas(address data_account_to_initialize, address payer) public {
        AccountMeta[3] metas = [
            AccountMeta({
                pubkey: data_account_to_initialize,
                is_signer: true, 
                is_writable: true}),
            AccountMeta({
                pubkey: payer,
                is_signer: true,
                is_writable: true}),
            AccountMeta({
                pubkey: address"11111111111111111111111111111111",
                is_writable: false,
                is_signer: false})
        ];

        Child.new{accounts: metas}(payer);        
  
        Child.use_metas();
    }
}

@program_id("Chi1d5XD6nTAp2EyaNGqMxZzUjh6NvhXRxbGHP3D1RaT")
contract Child {
    @payer(payer)
    constructor(address payer) {
        print("In child constructor");
    }

    function use_metas() pure public {
        print("I am using metas");
    }
}