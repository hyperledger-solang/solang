
        contract test_struct_parsing {
            struct Foo {
                bool x;
                int32 y;
            }

            function f() private {
                Foo a = Foo(true, true, true);
            }
        }