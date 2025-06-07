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

    function addingu64(uint64 a, uint64 b) public returns (uint64) {
        return a + b;
    }

    function addingu32(uint32 a, uint32 b) public returns (uint32) {
        return a + b;
    }
}