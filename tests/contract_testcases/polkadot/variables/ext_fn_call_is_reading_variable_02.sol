contract C {
    function ext_func_call() public payable {
        A a = new A();
        function() external payable func = a.a;
    }
}

contract A {
    function a() public payable {}
}

// ---- Expect: diagnostics ----
// warning: 4:37-41: local variable 'func' has been assigned, but never read
