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

    function additionu32 (uint32 a , uint32 b) public returns (uint32){
        return a+b;
    }

}
