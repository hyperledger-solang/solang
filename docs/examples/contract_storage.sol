contract hitcount {
    uint256 public counter = 1;

    function hit() public {
        counter++;
    }
}
