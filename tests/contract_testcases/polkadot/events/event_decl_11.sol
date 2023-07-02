abstract contract c {
    event foo(bool x, uint32 y, address indexed);
}

// ---- Expect: diagnostics ----
// error: 2:33-48: indexed event fields must have a name on polkadot
