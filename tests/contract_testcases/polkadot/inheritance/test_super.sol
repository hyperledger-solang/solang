contract super {

}
// ---- Expect: diagnostics ----
// error: 1:1-3:2: contracts without public storage or functions are not allowed on Polkadot. Consider declaring this contract abstract: 'abstract contract super'
// warning: 1:10-15: 'super' shadows name of a builtin
