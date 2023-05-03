
        contract c {
            event foo (bool indexed f1, bool indexed f2, bool indexed f3, bool indexed f4, bool indexed f5) anonymous;
        }
// ---- Expect: diagnostics ----
// error: 3:19-22: anonymous event definition for 'foo' has 5 indexed fields where 4 permitted
