contract MyTest {
    function foo() public pure returns (bytes memory) {
        bytes b1 = hex"41";
        bytes b2 = hex"41";

        b2.push(0x41);

        return (b1);
    }

    function foo2() public pure returns (uint64) {
        uint64[] a;
        a.push(20);
        return a[0];
    }
}
