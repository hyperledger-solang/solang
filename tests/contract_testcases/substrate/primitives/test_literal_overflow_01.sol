contract test {
            uint16 foo = 0x10000;
        }
// ----
// error (0-59): contracts without public storage or functions are not allowed on Substrate. Consider declaring this contract abstract: 'abstract contract test'
// error (41-48): value 65536 does not fit into type uint16.
