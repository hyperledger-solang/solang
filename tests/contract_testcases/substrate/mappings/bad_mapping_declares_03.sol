
        contract c {
            mapping(int => address) data;
            mapping(data => address) data2;
        }
// ---- Expect: diagnostics ----
// error: 4:21-25: 'data' is a contract variable
