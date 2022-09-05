
        abstract contract foo {
            enum e { e1, e2, e3 }
            e[1 / 0] x;
        }