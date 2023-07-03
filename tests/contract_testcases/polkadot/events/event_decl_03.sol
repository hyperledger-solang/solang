
        contract c {
            enum e { a1 }
            event e();
        }
// ---- Expect: diagnostics ----
// error: 4:19-20: e is already defined as an enum
// 	note 3:18-19: location of previous definition
