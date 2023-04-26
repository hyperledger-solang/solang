abstract contract test {
            uint x = 102;
            uint constant y = x + 5;
        }
// ----
// error (81-82): cannot read contract variable 'x' in constant expression
