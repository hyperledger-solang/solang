abstract contract test {
            int8 foo = 127;
        }
// ---- Expect: diagnostics ----
// warning: 2:13-27: storage variable 'foo' has been assigned, but never read
