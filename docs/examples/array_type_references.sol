abstract contract reference {
    function set_2(int8[4] a) private pure {
        a[2] = 102;
    }

    function foo() private {
        int8[4] val = [1, 2, 3, 4];

        set_2(val);

        // val was passed by reference, so was modified
        assert(val[2] == 102);
    }
}
