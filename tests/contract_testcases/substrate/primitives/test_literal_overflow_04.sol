abstract contract test {
            int8 foo = 127;
        }
// ----
// warning (37-51): storage variable 'foo' has been assigned, but never read
