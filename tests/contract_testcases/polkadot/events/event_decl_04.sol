
        contract c {
            event foo (mapping (bool => uint) x);
        }
// ---- Expect: diagnostics ----
// error: 3:24-48: mapping type is not permitted as event field
