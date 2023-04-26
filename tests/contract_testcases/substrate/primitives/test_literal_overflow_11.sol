abstract contract test {
            bytes4 foo = 0x00223344;
        }
// ----
// warning (37-60): storage variable 'foo' has been assigned, but never read
