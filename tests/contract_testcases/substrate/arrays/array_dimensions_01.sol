
        abstract contract foo {
            bool[-10 + 10] x;
        }
// ----
// error (50-58): zero size array not permitted
