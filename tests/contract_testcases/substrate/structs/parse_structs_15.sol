
        contract test_struct_parsing {
            struct Foo {
                bool x;
                int32 y;
            }

            function f() public {
                Foo a = Foo({ x: true, z: 1 });
            }
        }
// ---- Expect: diagnostics ----
// error: 9:40-41: struct 'Foo' has no field 'z'
