
        abstract contract c {
            modifier foo(bool x) {
                if (true) {
                    while (x) {
                        _;
                    }
                }
            }
        }
// ---- Expect: diagnostics ----
