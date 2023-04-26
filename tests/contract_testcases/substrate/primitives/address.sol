contract test {
            address  foo = 0x1844674_4073709551616;
        }
// ----
// error (0-77): contracts without public storage or functions are not allowed on Substrate. Consider declaring this contract abstract: 'abstract contract test'
// error (43-66): expected 'address', found integer
