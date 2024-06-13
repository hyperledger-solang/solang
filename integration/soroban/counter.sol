contract counter {
    uint64 public count = 10;

    function increment() public returns (uint64) {
        count += 1;
        return count;
    }

    function decrement() public returns (uint64) {
        count -= 1;
        return count;
    }
}
