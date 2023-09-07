contract foo {
    function test1() public returns (bytes) {
        bytes bs = new bytes(12);
        bs.writeInt32LE(-0x41424344, 0);
        bs.writeUint64LE(0x0102030405060708, 4);
        return bs;
    }

    function test2() public returns (bytes) {
        bytes bs = new bytes(34);
        bs.writeUint16LE(0x4142, 0);
        bs.writeAddress(address(this), 2);
        return bs;
    }

    function test3() public returns (bytes) {
        bytes bs = new bytes(9);
        bs.writeUint64LE(1, 2);
        return bs;
    }
}
