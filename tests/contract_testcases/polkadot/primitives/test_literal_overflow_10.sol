abstract contract test {
            bytes4 foo = 0x0122334455;
        }
// ---- Expect: diagnostics ----
// error: 2:26-38: hex literal 0x0122334455 must be 8 digits for type 'bytes4'
