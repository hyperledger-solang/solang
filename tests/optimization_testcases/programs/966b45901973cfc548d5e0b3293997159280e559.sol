contract foo {
    function get() public returns (bytes8) {
        return type(I).interfaceId;
    }
}

interface I {
    function bar(int) external;

    function baz(bytes) external returns (int);
}
