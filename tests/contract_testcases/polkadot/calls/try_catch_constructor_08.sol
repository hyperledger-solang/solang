contract A {
    function a() public payable returns (uint) {
        B b = new B();
        try b.b(0) {} catch Panic(uint code) {
            return code;
        }
        revert("didn't catch");
    }
}

contract B {
    function b(uint div) public pure returns (uint) {
        return 123 / div;
    }
}

// ---- Expect: diagnostics ----
