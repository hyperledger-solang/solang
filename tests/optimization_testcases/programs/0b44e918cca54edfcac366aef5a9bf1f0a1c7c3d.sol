library Library {
    uint256 internal constant STATIC = 42;
}

contract foo {
    function f() public returns (uint) {
        uint a = Library.STATIC;
        return a;
    }
}
