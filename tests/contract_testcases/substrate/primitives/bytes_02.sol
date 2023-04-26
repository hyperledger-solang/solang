contract Foo {
    function foo() public pure {
        uint16 i = 0x0_00AA;
        bytes3(0x0_00AA);
    }
}

// ----
// error (85-101): number of 1 bytes cannot be converted to type 'bytes3'
