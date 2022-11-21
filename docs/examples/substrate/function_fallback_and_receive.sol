contract test {
    int32 bar;

    function foo(int32 x) public {
        bar = x;
    }

    fallback() external {
        // execute if function selector does not match "foo(uint32)" and no value sent
    }

    receive() external payable {
        // execute if function selector does not match "foo(uint32)" and value sent
    }
}
