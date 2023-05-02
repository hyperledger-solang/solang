
        library c is x {
            fallback() internal {}
        }
// ---- Expect: diagnostics ----
// error: 2:22-23: library 'c' cannot have a base contract
// error: 2:22-23: 'x' not found
// error: 3:13-32: fallback not allowed in a library
