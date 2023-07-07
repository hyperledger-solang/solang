
        abstract contract test_struct_parsing {
            struct Foo {
                bool a;
                uint a;
            }
        }
// ---- Expect: diagnostics ----
// error: 5:22-23: struct 'Foo' has duplicate struct field 'a'
// 	note 4:17-23: location of previous declaration of 'a'
