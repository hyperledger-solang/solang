contract foo {
    uint256 public immutable bar;

    constructor(uint256 v) {
        bar = v;
    }

    function hit() public {
        // this is not permitted
        // bar++;
    }
}
