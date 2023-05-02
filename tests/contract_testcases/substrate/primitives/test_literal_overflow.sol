abstract contract test {
            uint8 foo = 300;
        }
// ---- Expect: diagnostics ----
// error: 2:25-28: value 300 does not fit into type uint8.
