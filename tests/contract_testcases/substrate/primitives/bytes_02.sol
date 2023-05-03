contract Foo {
    function foo() public pure {
        uint16 i = 0x0_00AA;
        bytes3(0x0_00AA);
    }
}

// ---- Expect: diagnostics ----
// error: 4:9-25: number of 1 bytes cannot be converted to type 'bytes3'
