
        contract test_struct_parsing {
            struct Foo {
                bool x;
                int32 y;
            }

            function f() public {
                Foo a = Foo(true, true, true);
            }
        }
// ----
// error (187-208): struct 'Foo' has 2 fields, not 3
