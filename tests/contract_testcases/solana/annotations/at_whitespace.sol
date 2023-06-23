@program_id("SeedHw4CsFsDEGu2AVwFM1toGXsbAJSKnb7kS8TrLxu")
contract Seed1 {

    @payer(payer)
    constructor(@ seed bytes seed, @bump bytes1 bump, @space uint64 space) {
        print("In Seed1 constructor");
    }

    function say_hello() pure public {
        print("Hello from Seed1");
    }
}

// ---- Expect: diagnostics ----
// error: 5:17-18: unrecognised token '@'
// error: 5:24-29: unrecognised token 'bytes', expected "(", ")", "++", ",", "--", ".", "[", "calldata", "case", "default", "leave", "memory", "revert", "storage", "switch", "{", identifier
