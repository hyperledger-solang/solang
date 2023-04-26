
        contract test_struct_parsing {
            struct Foo {
                bool x;
                Foo y;
            }
        }
// ----
// error (59-62): struct 'Foo' has infinite size
// 	note (105-110): recursive field 'y'
