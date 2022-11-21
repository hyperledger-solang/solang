contract example {
    function test() public {
        bytes runtime = type(other).runtimeCode;
    }
}

contract other {
    function foo() public returns (bool) {
        return true;
    }
}
