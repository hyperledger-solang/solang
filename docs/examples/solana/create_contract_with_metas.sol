import 'solana';

contract creator {

    @mutableSigner(data_account_to_initialize)
    @mutableSigner(payer)
    function create_with_metas() external {
        AccountMeta[3] metas = [
            AccountMeta({
                pubkey: tx.accounts.data_account_to_initialize.key,
                is_signer: true, 
                is_writable: true}),
            AccountMeta({
                pubkey: tx.accounts.payer.key,
                is_signer: true,
                is_writable: true}),
            AccountMeta({
                pubkey: address"11111111111111111111111111111111",
                is_writable: false,
                is_signer: false})
        ];

        Child.new{accounts: metas}();        
  
        Child.use_metas{accounts: []}();
    }
}

@program_id("Chi1d5XD6nTAp2EyaNGqMxZzUjh6NvhXRxbGHP3D1RaT")
contract Child {
    @payer(payer)
    constructor() {
        print("In child constructor");
    }

    function use_metas() pure public {
        print("I am using metas");
    }
}