contract foo {
    // make sure incrementer has a base contract with an empty constructor
    // is to check that the correct constructor is selected at emit time
    // https://github.com/hyperledger-solang/solang/issues/487
    constructor() {}
}

contract incrementer is foo {
    uint32 private value;

    /// Constructor that initializes the `int32` value to the given `init_value`.
    constructor(uint32 initvalue) {
        value = initvalue;
    }

    /// This increments the value by `by`.
    function inc(uint32 by) public {
        value += by;
    }

    /// Simply returns the current value of our `uint32`.
    function get() public view returns (uint32) {
        return value;
    }
}
