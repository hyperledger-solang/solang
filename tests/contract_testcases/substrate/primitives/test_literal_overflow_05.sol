abstract contract test {
            int8 foo = -128;
        }
// ---- Expect: diagnostics ----
// warning: 2:13-28: storage variable 'foo' has been assigned, but never read
