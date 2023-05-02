abstract contract test {
            uint8 foo = 255;
        }
// ---- Expect: diagnostics ----
// warning: 2:13-28: storage variable 'foo' has been assigned, but never read
