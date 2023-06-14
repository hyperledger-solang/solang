// RUN: --target solana --emit cfg

@program_id("SoLDxXQ9GMoa15i4NavZc61XGkas2aom4aNiWT6KUER")
contract Builder {
    Built other;
    // BEGIN-CHECK: Builder::Builder::function::build_this__address
    function build_this(address addr) external {
        // CHECK: constructor(no: 4) salt: value: gas:uint64 0 address: seeds: Built encoded buffer: %abi_encoded.temp.17 accounts: [3] [ struct { (deref (arg #0), true, false }, struct { (load (struct (subscript struct AccountInfo[] (builtin Accounts ())[uint32 1]) field 0)), true, true }, struct { (deref address 0x0, false, false } ]
        other = new Built{address: addr}("my_seed");
    }

    function call_that() public pure {
        other.say_this("Hold up! I'm calling!");
    }
}


@program_id("SoLGijpEqEeXLEqa9ruh7a6Lu4wogd6rM8FNoR7e3wY")
contract Built {
    @space(1024)
    @payer(payer_account)
    constructor(@seed bytes my_seed) {}
    // BEGIN-CHECK: solang_dispatch
    // CHECK: ty:struct AccountMeta[2] %metas.temp.10 = [2] [ struct { (load (struct (subscript struct AccountInfo[] (builtin Accounts ())[uint32 1]) field 0)), true, true }, struct { (builtin GetAddress ()), true, true } ]
    // The account metas should have the proper index in the AccountInfo array: 1

    function say_this(string text) public pure {
        print(text);
    }
}