contract C {
    function c() public {
        try new A() {} catch Panic(uint) {}
    }
}

contract A {
    function a() public pure {}
}

// ---- Expect: diagnostics ----
// error: 3:30-35: only catch 'Error' is supported, not 'Panic'
