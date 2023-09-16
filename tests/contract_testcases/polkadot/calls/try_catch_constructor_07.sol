contract C {
    function c() public {
        try new A() {} catch Panic(uint) {}
    }
}

contract A {
    function a() public pure {}
}

// ---- Expect: diagnostics ----
