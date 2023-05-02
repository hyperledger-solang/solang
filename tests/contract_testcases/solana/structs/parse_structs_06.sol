
        contract test_struct_parsing {
            struct Foo {
                bool x;
                Foo y;
            }
        }
// ---- Expect: diagnostics ----
// error: 3:20-23: struct 'Foo' has infinite size
// 	note 5:17-22: recursive field 'y'
