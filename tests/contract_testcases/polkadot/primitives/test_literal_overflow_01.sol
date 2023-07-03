contract test {
            uint16 foo = 0x10000;
        }
// ---- Expect: diagnostics ----
// error: 1:1-3:10: contracts without public storage or functions are not allowed on Polkadot. Consider declaring this contract abstract: 'abstract contract test'
// error: 2:26-33: value 65536 does not fit into type uint16.
