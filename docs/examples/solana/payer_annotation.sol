import 'solana';

@program_id("SoLDxXQ9GMoa15i4NavZc61XGkas2aom4aNiWT6KUER")
contract Builder {
    function build_this() external {
        // When calling a constructor from an external function, the data account for the contract
        // 'BeingBuilt' should be passed as the 'BeingBuilt_dataAccount' in the client code.
        BeingBuilt.new("my_seed");
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
        BeingBuilt.new{accounts: metas}("my_seed");


        // No accounts are needed in this call, so we pass an empty vector.
        BeingBuilt.say_this{accounts: []}("It's summertime!");
    }
}


@program_id("SoLGijpEqEeXLEqa9ruh7a6Lu4wogd6rM8FNoR7e3wY")
contract BeingBuilt {
    @space(1024)
    @payer(payer_account)
    constructor(@seed bytes my_seed) {}

    function say_this(string text) public pure {
        print(text);
    }
}