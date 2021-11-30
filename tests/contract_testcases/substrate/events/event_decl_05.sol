
        struct s {
            mapping (bool => uint) f1;
        }

        contract c {
            event foo (s x);
        }