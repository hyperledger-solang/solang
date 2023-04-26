
        enum e { a1 }
        abstract contract c {
            event e();
        }
// ----
// warning (71-72): e is already defined as an enum
// 	note (14-15): location of previous definition
