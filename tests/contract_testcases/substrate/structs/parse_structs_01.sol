
        contract test_struct_parsing {
            struct Foo {
                bool a;
                uint storage b;
            }
        }
// ----
// error (110-117): storage location 'storage' not allowed for struct field
