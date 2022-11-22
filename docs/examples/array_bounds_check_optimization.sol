contract c {
    function test() public returns (int256[]) {
        int256[] array = new int256[](3);
        array[1] = 1;
        return array;
    }
}
