
        contract test_struct_parsing {
            struct Foo {
                bool a;
                uint storage b;
            }
        }
// ---- Expect: diagnostics ----
// error: 5:22-29: storage location 'storage' not allowed for struct field
