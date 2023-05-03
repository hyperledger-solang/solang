
        contract c {
            event foo (bool indexed f1, bool indexed f2, bool indexed f3, bool indexed f4);
        }
// ---- Expect: diagnostics ----
// error: 3:19-22: event definition for 'foo' has 4 indexed fields where 3 permitted
