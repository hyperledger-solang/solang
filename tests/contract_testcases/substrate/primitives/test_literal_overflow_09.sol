abstract contract test {
            bytes4 foo = 0xf12233;
        }
// ---- Expect: diagnostics ----
// error: 2:26-34: hex literal 0xf12233 must be 8 digits for type 'bytes4'
