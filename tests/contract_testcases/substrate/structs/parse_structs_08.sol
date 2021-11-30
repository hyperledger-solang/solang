
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