contract C {
    function ext_func_call() public payable {
        A a = new A();
        function() external payable func = a.a;
        func();
    }
}

contract A {
    function a() public payable {}
}

// ---- Expect: diagnostics ----
