
        enum e { a1 }
        abstract contract c {
            event e();
        }
// ---- Expect: diagnostics ----
// warning: 4:19-20: e is already defined as an enum
// 	note 2:14-15: location of previous definition
