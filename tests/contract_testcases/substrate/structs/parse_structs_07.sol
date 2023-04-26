
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
// ----
// error (65-66): struct 's' has infinite size
// 	note (124-129): recursive field 'f3'
// error (157-159): struct 's2' has infinite size
// 	note (203-208): recursive field 'foo'
