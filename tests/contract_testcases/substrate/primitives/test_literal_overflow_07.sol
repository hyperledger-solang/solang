abstract contract test {
            uint8 foo = -1_30;
        }
// ----
// error (49-54): negative value -130 does not fit into type uint8. Cannot implicitly convert signed literal to unsigned type.
