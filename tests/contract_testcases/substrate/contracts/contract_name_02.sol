abstract contract test {
            bool test;
        }
// ---- Expect: diagnostics ----
// warning: 2:13-22: storage variable 'test' has never been used
// warning: 2:18-22: test is already defined as a contract name
// 	note 1:1-3:10: location of previous definition
