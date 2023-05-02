
        enum e { a1 }
        event e();
// ---- Expect: diagnostics ----
// error: 3:15-16: e is already defined as an enum
// 	note 2:14-15: location of previous definition
