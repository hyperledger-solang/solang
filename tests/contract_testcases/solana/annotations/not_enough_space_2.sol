contract Test3 {
    address public a;
    address public b;

    @payer(acc)
    @space(5+8*2)
    constructor(address c, address d) {
        a = c;
        b = d;
    }
}

// ---- Expect: diagnostics ----
// error: 6:12-17: contract requires at least 80 bytes of space
