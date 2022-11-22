contract dynamicarray {
    function test(uint32 size) public {
        int64[] memory a = new int64[](size);

        for (uint32 i = 0; i < size; i++) {
            a[i] = 1 << i;
        }

        assert(a.length == size);
    }
}
