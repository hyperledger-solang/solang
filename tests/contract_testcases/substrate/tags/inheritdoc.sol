
contract x {}

contract a {}

contract b is a {}

contract c is b {
    /// @inheritdoc a
    int v1;

    /// @inheritdoc x
    int v2;

}

// ---- Expect: diagnostics ----
// error: 12:21-22: base contract 'x' not found in tag '@inheritdoc'