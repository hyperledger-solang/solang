contract foo {
    function f(int64 n) public {
        unchecked {
            int64 j = n - 1;
        }
    }
}
