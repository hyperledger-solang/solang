abstract contract test {
            int64 foo = 1844674_4073709551616;
        }
// ----
// error (49-70): value 18446744073709551616 does not fit into type int64.
