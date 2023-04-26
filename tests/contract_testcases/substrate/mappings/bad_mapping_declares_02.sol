
        contract c {
            struct foo {
                int x;
            }
            mapping(foo => address) data;
        }
// ----
// error (104-107): key of mapping cannot be struct type
