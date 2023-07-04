contract test {
            address foo = address"5GBWmgdFAMqm8ZgAHGobqDqX6tjLxJhv53ygjNtaaAn3sje";
        }
// ---- Expect: diagnostics ----
// error: 1:1-3:10: contracts without public storage or functions are not allowed on Polkadot. Consider declaring this contract abstract: 'abstract contract test'
// error: 2:27-83: address literal 5GBWmgdFAMqm8ZgAHGobqDqX6tjLxJhv53ygjNtaaAn3sje incorrect length of 34
