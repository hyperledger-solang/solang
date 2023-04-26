abstract contract test {
            bytes4 foo = 0x0122334455;
        }
// ----
// error (50-62): hex literal 0x0122334455 must be 8 digits for type 'bytes4'
