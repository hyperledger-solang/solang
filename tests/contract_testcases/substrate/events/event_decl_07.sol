
        contract c {
            event foo (bool indexed f1, bool indexed f2, bool indexed f3, bool indexed f4);
        }
// ----
// error (40-43): event definition for 'foo' has 4 indexed fields where 3 permitted
