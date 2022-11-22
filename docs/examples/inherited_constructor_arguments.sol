contract a is b(1) {
    constructor() {}
}

contract b is c(2) {
    int256 foo;

    function func2(int256 i) public {}

    constructor(int256 j) {}
}

contract c {
    int256 bar;

    constructor(int32 j) {}

    function func1() public {}
}
