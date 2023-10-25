// RUN: --target solana --emit cfg

@program_id("SoLDxXQ9GMoa15i4NavZc61XGkas2aom4aNiWT6KUER")
contract Builder {
    // BEGIN-CHECK: Builder::Builder::function::build_this
    function build_this() external {
        // CHECK: external call::regular address:address 0x69be884fd55a2306354c305323cc6b7ce91768be33d32a021155ef608806bcb payload:%abi_encoded.temp.17 value:uint64 0 gas:uint64 0 accounts:[3] [ struct { (load (struct (subscript struct AccountInfo[] (builtin Accounts ())[uint32 3]) field 0)), true, false }, struct { (load (struct (subscript struct AccountInfo[] (builtin Accounts ())[uint32 2]) field 0)), true, true }, struct { (load (struct (subscript struct AccountInfo[] (builtin Accounts ())[uint32 0]) field 0)), false, false } ] seeds: contract|function:(1, 4) flags:
        Built.new("my_seed");
    }

    function call_that() external view {
        Built.say_this("Hold up! I'm calling!");
    }
}


@program_id("SoLGijpEqEeXLEqa9ruh7a6Lu4wogd6rM8FNoR7e3wY")
contract Built {
    @space(1024)
    @payer(payer_account)
    constructor(@seed bytes my_seed) {}
    // BEGIN-CHECK: solang_dispatch
    // CHECK: ty:struct AccountInfo %temp.10 = (subscript struct AccountInfo[] (builtin Accounts ())[uint32 1])
	// CHECK: ty:struct AccountInfo %temp.11 = (subscript struct AccountInfo[] (builtin Accounts ())[uint32 0])
	// CHECK: ty:struct AccountMeta[2] %metas.temp.9 = [2] [ struct { (load (struct %temp.10 field 0)), true, true }, struct { (load (struct %temp.11 field 0)), true, true } ]

    // The account metas should have the proper index in the AccountInfo array: 1

    function say_this(string text) public pure {
        print(text);
    }
}