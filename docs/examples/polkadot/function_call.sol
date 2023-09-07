contract A {
    function test(B v) public {
        // the following four lines are equivalent to "uint32 res = v.foo(3,5);"

        // Note that the signature is only hashed and not parsed. So, ensure that the
        // arguments are of the correct type.
        bytes data = abi.encodeWithSignature(
            "foo(uint32,uint32)",
            uint32(3),
            uint32(5)
        );

        (bool success, bytes rawresult) = address(v).call(data);

        assert(success == true);

        uint32 res = abi.decode(rawresult, (uint32));

        assert(res == 8);
    }
}

contract B {
    function foo(uint32 a, uint32 b) pure public returns (uint32) {
        return a + b;
    }
}
