
        contract c {
            struct foo {
                int x;
            }
            mapping(foo => address) data;
        }
// ---- Expect: diagnostics ----
// error: 6:21-24: key of mapping cannot be struct type
