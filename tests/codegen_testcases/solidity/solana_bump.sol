// RUN: --target solana --emit cfg

contract C1 {
    @payer(payer)
    @space(57)
    @bump(25)
    constructor(@seed bytes my_seed) {
        print("In C1");
    }
    // BEGIN-CHECK: solang_dispatch
    // 25 must be the last seed in the call.
    // CHECK: external call::regular address:address 0x0 payload:%instruction.temp.15 value:uint64 0 gas:uint64 0 accounts:%metas.temp.11 seeds:[1] [ [2] [ bytes(%my_seed), bytes((alloc bytes uint32 1 "\u{19}")) ] ] contract|function:_ flags:
}

contract C2 {
    @payer(payer)
    @space(57)
    @seed("apple")
    @bump(12)
    @seed("pine_tree")
    constructor(@seed bytes my_seed) {
        print("In C2");
    }
    // BEGIN-CHECK: solang_dispatch
    // 12 must be the last seed in the call.
    // CHECK: external call::regular address:address 0x0 payload:%instruction.temp.27 value:uint64 0 gas:uint64 0 accounts:%metas.temp.23 seeds:[1] [ [4] [ (alloc slice bytes1 uint32 5 "apple"), (alloc slice bytes1 uint32 9 "pine_tree"), bytes(%my_seed), bytes((alloc bytes uint32 1 "\u{c}")) ] ] contract|function:_ flags:
}

contract C3 {
    @payer(payer)
    @space(57)
    @seed("pineapple")
    @seed("avocado")
    constructor(@bump uint8 bp, @seed bytes my_seed) {
        print("In C3");
    }
    // BEGIN-CHECK: solang_dispatch
    // bp must be the last seed in the call
    // CHECK: external call::regular address:address 0x0 payload:%instruction.temp.40 value:uint64 0 gas:uint64 0 accounts:%metas.temp.36 seeds:[1] [ [4] [ (alloc slice bytes1 uint32 9 "pineapple"), (alloc slice bytes1 uint32 7 "avocado"), bytes(%my_seed), bytes(bytes from:bytes1 (%bp)) ] ] contract|function:_ flags:
}
