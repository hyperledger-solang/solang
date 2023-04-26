
        abstract contract foo {
            struct bar {
                int32 x;
            }
            bar[1 % 0] x;
        }
// ----
// error (113-118): divide by zero
