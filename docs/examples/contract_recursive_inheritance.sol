contract a is b {
    constructor() {}
}

contract b is c {
    int256 foo;

    function func2() public {}

    constructor() {}
}

contract c {
    int256 bar;

    constructor() {}

    function func1() public {}
}
