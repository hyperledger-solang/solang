contract Test2 {
    address public a;
    address public b;

    @payer(acc)
    @space(5<<64)
    constructor(address c, address d) {
        a = c;
        b = d;
    }
}

// ---- Expect: diagnostics ----
// error: 6:12-17: Solana's runtime does not permit accounts larger than 10 MB
// error: codegen: value 92233720368547758208 does not fit into type uint64.