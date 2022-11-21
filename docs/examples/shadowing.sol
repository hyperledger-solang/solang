contract test {
    uint256 foo = 102;
    uint256 bar;

    function foobar() public {
        // AVOID: this shadows the contract storage variable foo
        uint256 foo = 5;
    }
}
