abstract contract test {
            bytes4 foo = 0xf12233;
        }
// ----
// error (50-58): hex literal 0xf12233 must be 8 digits for type 'bytes4'
