abstract contract test {
            address foo = 0x5b0Ddf2835f0A76c96D6113D47F6482e51a55487;
        }
// ---- Expect: diagnostics ----
// error: 2:27-69: ethereum address literal '0x5b0Ddf2835f0A76c96D6113D47F6482e51a55487' not supported on target Polkadot
