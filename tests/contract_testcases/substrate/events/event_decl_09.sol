
        contract c {
            event foo (bool indexed f1, bool indexed f2, bool indexed f3, bool indexed f4, bool indexed f5) anonymous;
        }
// ----
// error (40-43): anonymous event definition for 'foo' has 5 indexed fields where 4 permitted
