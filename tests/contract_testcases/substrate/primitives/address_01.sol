contract test {
            address foo = 0xa368df6dfcd5ba7b0bc108af09e98e4655e35a2c3b2e2d5e3eae6c6f7cd8d2d4;
        }
// ----
// error (0-119): contracts without public storage or functions are not allowed on Substrate. Consider declaring this contract abstract: 'abstract contract test'
// error (42-108): expected 'address', found integer
