import 'solana';

@program_id("SoLDxXQ9GMoa15i4NavZc61XGkas2aom4aNiWT6KUER")
contract Builder {
    BeingBuilt other;
    function build_this(address addr) external {
        // When calling a constructor from an external function, the only call argument needed
        // is the data account. The compiler automatically passes the necessary accounts to the call.
        other = new BeingBuilt{address: addr}("my_seed");
    }

    function build_that(address data_account, address payer_account) public {
        // In non-external functions, developers need to manually create the account metas array.
        // The order of the accounts must match the order from the BeingBuilt IDL file for the "new"
        // instruction.
        AccountMeta[3] metas = [
            AccountMeta({
                pubkey: data_account, 
                is_signer: true, 
                is_writable: true
                }),
            AccountMeta({
                pubkey: payer_account, 
                is_signer: true, 
                is_writable: true
                }),
            AccountMeta({
                pubkey: address"11111111111111111111111111111111", 
                is_writable: false,
                is_signer: false
                })
        ];
        other = new BeingBuilt{accounts: metas}("my_seed");
    }
}


@program_id("SoLGijpEqEeXLEqa9ruh7a6Lu4wogd6rM8FNoR7e3wY")
contract BeingBuilt {
    @seed(my_seed)
    @space(1024)
    @payer(payer_account)
    constructor(bytes my_seed) {}

    function say_this(string text) public pure {
        print(text);
    }
}