
        contract c {
            enum e { a1 }
            event e();
        }
// ----
// error (66-67): e is already defined as an enum
// 	note (39-40): location of previous definition
