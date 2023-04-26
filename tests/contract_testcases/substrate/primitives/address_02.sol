contract test {
            address foo = address"5GBWmgdFAMqm8ZgAHGobqDqX6tjLxJhv53ygjNtaaAn3sje";
        }
// ----
// error (0-109): contracts without public storage or functions are not allowed on Substrate. Consider declaring this contract abstract: 'abstract contract test'
// error (42-98): address literal 5GBWmgdFAMqm8ZgAHGobqDqX6tjLxJhv53ygjNtaaAn3sje incorrect length of 34
