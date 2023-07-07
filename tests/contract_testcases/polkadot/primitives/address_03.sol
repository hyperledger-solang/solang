contract test {
            address foo = address"5GBWmgdFAMqm8ZgAHGobqDqX6tjLxJhv53ygjNtaaAn3sj%Z";
        }
// ---- Expect: diagnostics ----
// error: 1:1-3:10: contracts without public storage or functions are not allowed on Polkadot. Consider declaring this contract abstract: 'abstract contract test'
// error: 2:73: address literal 5GBWmgdFAMqm8ZgAHGobqDqX6tjLxJhv53ygjNtaaAn3sj%Z invalid character '%'
