contract test {
    int32 bar;

    function foo(int32 x) public {
        bar = x;
    }

    fallback(bytes calldata input) external returns (bytes memory ret) {
        // execute if function selector does not match "foo(uint32)" and no value sent
	ret = "testdata";
    }

    receive() external payable {
        // execute if function selector does not match "foo(uint32)" and value sent
    }
}
