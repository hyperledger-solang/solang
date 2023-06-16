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
// error: 5:17-18: unrecognised token '@', expected "(", ")", ",", "[", "address", "bool", "byte", "bytes", "case", "default", "false", "function", "leave", "mapping", "payable", "string", "switch", "true", "type", Bytes, Int, Uint, address, annotation, hexnumber, hexstring, identifier, number, rational, string