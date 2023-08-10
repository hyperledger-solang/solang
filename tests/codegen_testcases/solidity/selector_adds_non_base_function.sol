// RUN: --target polkadot --emit cfg

// This test makes sure that we're not including the function a in contract C

// CHECK-ABSENT: C::A::function::a

contract C {
    function ext_func_call(uint128 amount) public payable {
        A a = new A();
        (bool ok, bytes b) = address(a).call(
            bytes4(A.a.selector)
        );
	b = abi.encodeCall(A.a);
    }
}

contract A {
    function a() public pure {}
}
