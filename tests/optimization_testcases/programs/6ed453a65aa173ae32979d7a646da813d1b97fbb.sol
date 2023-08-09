contract foo {
    uint private val = 0;

    function inc() public {
        val += 1;
    }

    function get() public returns (uint) {
        return val;
    }

    function strange() public {
        return inc();
    }
}
