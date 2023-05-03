
        contract c {
            mapping(uint[] => address) data;
        }
// ---- Expect: diagnostics ----
// error: 3:21-27: key of mapping cannot be array type
