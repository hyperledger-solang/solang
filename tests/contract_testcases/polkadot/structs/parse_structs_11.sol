
        contract test_struct_parsing {
            struct Foo {
                bool x;
                int32 y;
            }

            function f() public {
                Foo a = Foo();
            }
        }
// ---- Expect: diagnostics ----
// error: 9:25-30: struct 'Foo' has 2 fields, not 0
