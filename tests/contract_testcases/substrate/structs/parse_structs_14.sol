
        contract test_struct_parsing {
            struct Foo {
                bool x;
                int32 y;
            }

            function f() public {
                Foo a = Foo({ x: true, y: 1, z: 2 });
            }
        }
// ----
// error (187-215): struct 'Foo' has 2 fields, not 3
