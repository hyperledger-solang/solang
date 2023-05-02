
        contract test_struct_parsing {
            struct Foo {
                boolean x;
            }
        }
// ---- Expect: diagnostics ----
// error: 4:17-24: type 'boolean' not found
