abstract contract test {
            address foo = address"5GBWmgdFAMqm8ZgAHGobqDqX6tjLxJhv53ygjNtaaAn3sjeZ";
        }
// ---- Expect: diagnostics ----
// warning: 2:13-84: storage variable 'foo' has been assigned, but never read
