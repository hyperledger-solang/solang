contract noargsconstructor {
    uint64 public count = 1;

    constructor() {
        count += 1;
    }

    function get() public view returns (uint64) {
        return count;
    }
}
