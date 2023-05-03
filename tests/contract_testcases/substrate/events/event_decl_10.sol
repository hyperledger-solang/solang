
        abstract contract c {
            event foo (bool indexed f1, bool indexed f2, bool indexed f3, bool indexed f4) anonymous;
        }
// ---- Expect: diagnostics ----
