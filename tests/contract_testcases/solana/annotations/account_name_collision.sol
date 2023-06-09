@program_id("SoLDxXQ9GMoa15i4NavZc61XGkas2aom4aNiWT6KUER")
contract Builder {
    BeingBuilt other;
    
    @payer(payer_account)
    constructor(address addr) {
        other = new BeingBuilt{address: addr}("abc");
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

// ---- Expect: diagnostics ----
// warning: 3:5-21: storage variable 'other' has been assigned, but never read
// error: 5:5-26: account name collision encountered. Calling a function that requires an account whose name is also defined in the current function will create duplicate names in the IDL. Please, rename one of the accounts
// 	note 16:5-26: other declaration