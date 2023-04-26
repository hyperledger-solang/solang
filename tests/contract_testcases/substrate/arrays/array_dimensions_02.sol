
        abstract contract foo {
            bool[1 / 10] x;
        }
// ----
// error (50-56): zero size array not permitted
