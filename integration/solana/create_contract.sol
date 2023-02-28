import 'solana';

contract creator {
    Child public c;

    function create_child(address child, address payer) public {
        print("Going to create child");
        c = new Child{address: child}(payer);

        c.say_hello();
    }

    function create_seed1(address child, address payer, bytes seed, bytes1 bump, uint64 space) public {
        print("Going to create Seed1");
        Seed1 s = new Seed1{address: child}(payer, seed, bump, space);

        s.say_hello();
    }

    function create_seed2(address child, address payer, bytes seed, uint32 space) public {
        print("Going to create Seed2");
        new Seed2{address: child}(payer, seed, space);
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

@program_id("SeedHw4CsFsDEGu2AVwFM1toGXsbAJSKnb7kS8TrLxu")
contract Seed1 {

    @payer(payer)
    @seed(seed)
    @bump(bump)
    @space(space)
    constructor(address payer, bytes seed, bytes1 bump, uint64 space) {
        print("In Seed1 constructor");
    }

    function say_hello() pure public {
        print("Hello from Seed1");
    }
}

@program_id("Seed23VDZ9HFCfKvFwmemB6dpi25n5XjZdP52B2RUmh")
contract Seed2 {
    bytes my_seed;

    @payer(payer)
    @seed("sunflower")
    @seed(seed)
    @space(space + 23)
    constructor(address payer, bytes seed, uint64 space) {
        my_seed = seed;

        print("In Seed2 constructor");
    }

    function check() public view {
        address pda = create_program_address([ "sunflower", my_seed ], tx.program_id);

        if (pda == address(this)) {
            print("I am PDA.");
        }
    }
}