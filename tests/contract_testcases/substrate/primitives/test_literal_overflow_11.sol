abstract contract test {
            bytes4 foo = 0x00223344;
        }
// ---- Expect: diagnostics ----
// warning: 2:13-36: storage variable 'foo' has been assigned, but never read
