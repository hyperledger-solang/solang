
        contract c {
            event foo (mapping (bool => uint) x);
        }
// ----
// error (45-69): mapping type is not permitted as event field
