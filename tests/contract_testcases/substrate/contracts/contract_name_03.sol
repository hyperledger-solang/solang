abstract contract test {
            struct test { bool a; }
        }
// ---- Expect: diagnostics ----
// warning: 2:20-24: test is already defined as a contract name
// 	note 1:1-3:10: location of previous definition
