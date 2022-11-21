abstract contract x {
    constructor(address foobar) {
        require(foobar != address(0), "foobar must a valid address");
    }
}
