abstract contract test {
            uint x = y + 102;
            uint y = 102;
        }
// ---- Expect: diagnostics ----
// warning: 2:13-29: storage variable 'x' has been assigned, but never read
