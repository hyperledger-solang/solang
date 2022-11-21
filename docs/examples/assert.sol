abstract contract x {
    constructor(address foobar) {
        if (foobar == address(0)) {
            revert("foobar must a valid address");
        }
    }
}
