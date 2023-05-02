
        contract c {
            s z;
        }

        struct s {
            bool f1;
            int32 f2;
            s2 f3;
        }

        struct s2 {
            bytes4 selector;
            s foo;
        }
// ---- Expect: diagnostics ----
// error: 6:16-17: struct 's' has infinite size
// 	note 9:13-18: recursive field 'f3'
// error: 12:16-18: struct 's2' has infinite size
// 	note 14:13-18: recursive field 'foo'
