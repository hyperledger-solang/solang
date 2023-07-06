contract test {
    int8 foo = -129;
}
// ---- Expect: diagnostics ----
// error: 1:1-3:2: contracts without public storage or functions are not allowed on Polkadot. Consider declaring this contract abstract: 'abstract contract test'
// error: 2:16-20: value -129 does not fit into type int8.
