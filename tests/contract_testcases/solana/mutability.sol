contract c {
    function test2(int256[] storage A, int256[] storage B)
        internal
        pure
        returns (int256[] storage, int256[] storage)
    {
        int256[] storage x;
        int256[] storage y;
        (x, y) = (A, B);
        return (x, y);
    }
}

// ---- Expect: diagnostics ----
