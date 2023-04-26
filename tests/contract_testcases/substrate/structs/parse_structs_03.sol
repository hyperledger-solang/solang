
        contract test_struct_parsing {
            struct Foo {
                bool memory a;
                uint calldata b;
            }
        }
// ----
// error (86-92): storage location 'memory' not allowed for struct field
// error (117-125): storage location 'calldata' not allowed for struct field
