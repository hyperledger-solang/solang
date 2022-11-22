contract a is b {
    constructor(int256 i) b(i + 2) {}
}

contract b is c {
    int256 foo;

    function func2() public {}

    constructor(int256 j) c(int32(j + 3)) {}
}

contract c {
    int256 bar;

    constructor(int32 k) {}

    function func1() public {}
}
