
        contract foo {
            struct bar {
                int32 x;
            }
            bar[1 % 0] x;
        }