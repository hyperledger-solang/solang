
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