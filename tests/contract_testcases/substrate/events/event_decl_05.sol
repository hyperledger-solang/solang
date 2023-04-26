
        struct s {
            mapping (bool => uint) f1;
        }

        contract c {
            event foo (s x);
        }
// ----
// error (114-117): mapping type is not permitted as event field
