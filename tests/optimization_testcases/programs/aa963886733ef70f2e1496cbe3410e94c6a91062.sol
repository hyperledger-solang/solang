contract c {
    address factory;
    int decimals;
    int[2] arr1;
    int[2] arr2;

    constructor() {
        int[2] storage x;

        (x, factory, decimals) = foo();
        x[0] = 2;
    }

    function foo() internal view returns (int[2] storage, address, int) {
        return (arr2, address(2), 5);
    }

    function bar() public view {
        require(factory == address(2), "address wrong");
        require(decimals == 5, "int wrong");
        require(arr2[0] == 2, "array wrong");
    }
}
