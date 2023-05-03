uint constant test = 5; contract test {}
// ---- Expect: diagnostics ----
// error: 1:15-19: test is already defined as a contract name
// 	note 1:25-41: location of previous definition
