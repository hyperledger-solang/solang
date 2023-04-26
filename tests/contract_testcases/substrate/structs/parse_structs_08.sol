
        contract con {
            struct Foo {
                uint256 foo;
            }

            struct Bar {
                Foo foo;
            }

            struct Baz {
                Foo foo;
                Bar bar;
            }
        }
// ----
// error (18-21): contract name 'con' is reserved file name on Windows
