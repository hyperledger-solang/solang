abstract contract test {
            int64 foo = 1844674_4073709551616;
        }
// ---- Expect: diagnostics ----
// error: 2:25-46: value 18446744073709551616 does not fit into type int64.
