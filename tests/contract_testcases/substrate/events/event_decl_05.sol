
        struct s {
            mapping (bool => uint) f1;
        }

        contract c {
            event foo (s x);
        }
// ---- Expect: diagnostics ----
// error: 7:24-27: mapping type is not permitted as event field
