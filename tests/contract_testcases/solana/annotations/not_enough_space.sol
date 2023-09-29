contract Test {
    address a;
    address b;

    @payer(acc)
    @space(5)
    constructor(address c, address d) {
        a = c;
        b = d;
    }
}

// ---- Expect: diagnostics ----
// warning: 2:5-14: storage variable 'a' has been assigned, but never read
// warning: 3:5-14: storage variable 'b' has been assigned, but never read
// error: 6:12-13: contract requires at least 80 bytes of space
