
        contract test_struct_parsing {
            struct Foo {
                bool a;
                uint calldata b;
            }
        }
// ---- Expect: diagnostics ----
// error: 5:22-30: storage location 'calldata' not allowed for struct field
