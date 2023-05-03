abstract contract test {
            enum test { a}
        }
// ---- Expect: diagnostics ----
// warning: 2:18-22: test is already defined as a contract name
// 	note 1:1-3:10: location of previous definition
