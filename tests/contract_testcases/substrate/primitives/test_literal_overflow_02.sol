contract test {
            int8 foo = 0x8_0;
        }
// ----
// error (0-55): contracts without public storage or functions are not allowed on Substrate. Consider declaring this contract abstract: 'abstract contract test'
// error (39-44): value 128 does not fit into type int8.
