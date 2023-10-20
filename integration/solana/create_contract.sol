import 'solana';

contract creator {

    function create_child() external {
        print("Going to create child");
        Child.new();

        Child.say_hello();
    }

    function create_seed1(bytes seed, bytes1 bump, uint64 space) external {
        print("Going to create Seed1");
        Seed1.new(seed, bump, space);

        Seed1.say_hello();
    }

    function create_seed2(bytes seed, uint32 space) external {
        print("Going to create Seed2");

        Seed2.new(seed, space);
    }

    @mutableSigner(child)
    @mutableSigner(payer)
    function create_child_with_metas() external {
        print("Going to create child with metas");
        AccountMeta[3] metas = [
            AccountMeta({pubkey: tx.accounts.child.key, is_signer: true, is_writable: true}),
            AccountMeta({pubkey: tx.accounts.payer.key, is_signer: true, is_writable: true}),
            AccountMeta({pubkey: address"11111111111111111111111111111111", is_writable: false, is_signer: false})
        ];

        Child.new{accounts: metas}();
        Child.use_metas();
    }

    function create_without_annotation() external {
        MyCreature.new();
        MyCreature.say_my_name{accounts: []}();
    }

    @signer(my_signer)
    function call_with_signer() view external {
        require(tx.accounts.my_signer.is_signer, "the signer must sign the transaction");
        print("Signer found");
    }
}

@program_id("Chi1d5XD6nTAp2EyaNGqMxZzUjh6NvhXRxbGHP3D1RaT")
contract Child {
    @payer(payer)
    @space(511 + 7)
    constructor() {
        assert(tx.accounts.payer.is_signer);
        assert(tx.accounts.payer.is_writable);
        print("In child constructor");
    }

    function say_hello() pure public {
        print("Hello there");
    }

    function use_metas() pure public {
        print("I am using metas");
    }
}

@program_id("SeedHw4CsFsDEGu2AVwFM1toGXsbAJSKnb7kS8TrLxu")
contract Seed1 {
    bytes saved_seed;
    bytes1 saved_bump;

    @payer(payer)
    constructor(@seed bytes seed, @bump bytes1 bump, @space uint64 space) {
        print("In Seed1 constructor");
        saved_seed = seed;
        saved_bump = bump;
    }

    function say_hello() pure public {
        print("Hello from Seed1");
    }

    @account(creator_program_id)
    function sign() view external {
        AccountMeta[1] metas = [
            AccountMeta({pubkey: tx.accounts.dataAccount.key, is_signer: true, is_writable: false})
        ];

        creator.call_with_signer{seeds: [ [ saved_seed, saved_bump ] ],
        accounts: metas, 
        program_id: tx.accounts.creator_program_id.key}();
    }
}

@program_id("Seed23VDZ9HFCfKvFwmemB6dpi25n5XjZdP52B2RUmh")
contract Seed2 {
    bytes my_seed;

    @payer(payer)
    @seed("sunflower")
    constructor(@seed bytes seed, @space uint64 space) {
        my_seed = seed;

        print("In Seed2 constructor");
    }

    function check() public view {
        address pda = create_program_address([ "sunflower", my_seed ], address(this));

        if (pda == tx.accounts.dataAccount.key) {
            print("I am PDA.");
        }
    }

    @account(creator_program_id)
    function sign() view external {
        bytes[2][1] seeds = [ [ "sunflower", my_seed ] ];

        sign2(seeds, tx.accounts.dataAccount.key, tx.accounts.creator_program_id.key);
    }

    function sign2(bytes[2][1] seeds, address child, address creator_program_id) view internal {
        AccountMeta[1] metas = [
            AccountMeta({pubkey: child, is_signer: true, is_writable: false})
        ];

        creator.call_with_signer{seeds: seeds, accounts: metas, program_id: creator_program_id}();
    }

    @account(pdaSigner)
    @account(creatorId)
    function pda_sign(uint8 bump) external view {
        (address pda, bytes1 _bump) = try_find_program_address(["mint_authority"], address(this));
        assert(bump == _bump);
        assert(pda == tx.accounts.pdaSigner.key);

        AccountMeta[1] metas = [
            AccountMeta({pubkey: tx.accounts.pdaSigner.key, is_signer: true, is_writable: false})
        ];

        creator.call_with_signer{accounts: metas, seeds: [["mint_authority", abi.encode(_bump)]], program_id: tx.accounts.creatorId.key}();
    }
}

@program_id("8gTkAidfM82u3DGbKcZpHwL5p47KQA16MDb4WmrHdmF6")
contract MyCreature {
    constructor() {
        print("In child constructor");
    }

    function say_my_name() public pure {
        print("say_my_name");
    }
}