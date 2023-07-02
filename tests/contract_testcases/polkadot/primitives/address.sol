contract test {
            address  foo = 0x1844674_4073709551616;
        }
// ---- Expect: diagnostics ----
// error: 1:1-3:10: contracts without public storage or functions are not allowed on Polkadot. Consider declaring this contract abstract: 'abstract contract test'
// error: 2:28-51: expected 'address', found integer
