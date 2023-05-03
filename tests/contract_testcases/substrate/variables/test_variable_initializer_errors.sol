abstract contract test {
            uint x = 102;
            uint constant y = x + 5;
        }
// ---- Expect: diagnostics ----
// error: 3:31-32: cannot read contract variable 'x' in constant expression
