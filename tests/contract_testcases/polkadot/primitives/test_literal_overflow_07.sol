abstract contract test {
            uint8 foo = -1_30;
        }
// ---- Expect: diagnostics ----
// error: 2:25-30: negative value -130 does not fit into type uint8. Cannot implicitly convert signed literal to unsigned type.
