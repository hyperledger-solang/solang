contract test {
    int256 a;

    // this function reads a twice; this can be reduced to one load
    function redundant_load() public returns (int256) {
        return a + a;
    }

    // this function writes to contract storage thrice. This can be reduced to one
    function redundant_store() public {
        delete a;
        a = 1;
        a = 2;
    }
}
