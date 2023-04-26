abstract contract test {
            uint x = y + 102;
            uint constant y = 102;
        }
// ----
// warning (37-53): storage variable 'x' has been assigned, but never read
