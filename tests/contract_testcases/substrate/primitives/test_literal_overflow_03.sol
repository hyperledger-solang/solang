contract test {
            int8 foo = -129;
        }
// ---- Expect: diagnostics ----
// error: 1:1-3:10: contracts without public storage or functions are not allowed on Substrate. Consider declaring this contract abstract: 'abstract contract test'
// error: 2:24-28: value -129 does not fit into type int8.
