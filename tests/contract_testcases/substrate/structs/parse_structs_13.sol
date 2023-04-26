
        contract test_struct_parsing {
            struct Foo {
                bool x;
                int32 y;
            }

            function f() public {
                Foo a = Foo({ });
            }
        }
// ----
// error (187-195): struct 'Foo' has 2 fields, not 0
