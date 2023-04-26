abstract contract test {
            uint8 foo = 255;
        }
// ----
// warning (37-52): storage variable 'foo' has been assigned, but never read
