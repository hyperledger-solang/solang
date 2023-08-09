contract C {
    uint256 public constant STATIC = 42;
}

contract foo {
    function f() public returns (uint) {
        uint a = C.STATIC;
        return a;
    }
}
