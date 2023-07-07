contract test {
            address foo = 0xa368df6dfcd5ba7b0bc108af09e98e4655e35a2c3b2e2d5e3eae6c6f7cd8d2d4;
        }
// ---- Expect: diagnostics ----
// error: 1:1-3:10: contracts without public storage or functions are not allowed on Polkadot. Consider declaring this contract abstract: 'abstract contract test'
// error: 2:27-93: expected 'address', found integer
