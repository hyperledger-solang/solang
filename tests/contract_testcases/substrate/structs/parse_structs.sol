
        abstract contract test_struct_parsing {
            struct Foo {
                bool a;
                uint a;
            }
        }
// ----
// error (119-120): struct 'Foo' has duplicate struct field 'a'
// 	note (90-96): location of previous declaration of 'a'
