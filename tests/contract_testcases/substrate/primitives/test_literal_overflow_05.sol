abstract contract test {
            int8 foo = -128;
        }
// ----
// warning (37-52): storage variable 'foo' has been assigned, but never read
