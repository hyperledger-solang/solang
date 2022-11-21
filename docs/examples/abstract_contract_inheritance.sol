abstract contract a is b {
    constructor() {}
}

contract b {
    int256 public j;

    constructor(int256 _j) {}
}

contract c is a {
    int256 public k;

    constructor(int256 k) b(k * 2) {}
}
