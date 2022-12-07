enum E {
    v1,
    v2
}
struct S {
    int256 i;
    bool b;
    address a;
}

contract C {
    // foo_
    function foo() public pure {}

    // foo_uint256_addressArray2Array
    function foo(uint256 i, address[2][] memory a) public pure {}

    // foo_uint8Array2__int256_bool_address
    function foo(E[2] memory e, S memory s) public pure {}
}
