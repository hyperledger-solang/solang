
        contract test_struct_parsing {
            struct Foo {
            }
        }
// ---- Expect: diagnostics ----
// error: 3:20-23: struct definition for 'Foo' has no fields
