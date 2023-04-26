contract test {
            int8 foo = -129;
        }
// ----
// error (0-54): contracts without public storage or functions are not allowed on Substrate. Consider declaring this contract abstract: 'abstract contract test'
// error (39-43): value -129 does not fit into type int8.
