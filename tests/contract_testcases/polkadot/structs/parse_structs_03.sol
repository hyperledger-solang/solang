
        contract test_struct_parsing {
            struct Foo {
                bool memory a;
                uint calldata b;
            }
        }
// ---- Expect: diagnostics ----
// error: 4:22-28: storage location 'memory' not allowed for struct field
// error: 5:22-30: storage location 'calldata' not allowed for struct field
