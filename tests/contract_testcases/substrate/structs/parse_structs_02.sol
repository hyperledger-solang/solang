
        contract test_struct_parsing {
            struct Foo {
                bool a;
                uint calldata b;
            }
        }
// ----
// error (110-118): storage location 'calldata' not allowed for struct field
