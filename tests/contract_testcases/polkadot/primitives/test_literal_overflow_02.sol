contract test {
            int8 foo = 0x8_0;
        }
// ---- Expect: diagnostics ----
// error: 1:1-3:10: contracts without public storage or functions are not allowed on Polkadot. Consider declaring this contract abstract: 'abstract contract test'
// error: 2:24-29: value 128 does not fit into type int8.
