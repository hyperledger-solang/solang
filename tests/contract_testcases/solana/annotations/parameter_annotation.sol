contract bar {
    @space(511 + 102)
    constructor(@payer address addr) {}

    function hello() public returns (bool) {
        return true;
    }
}

contract beerBar {
    @bump(52)
    @payer(addr)
    constructor(@space uint64 d, @space uint8 f) {}

    function hello() public returns (bool) {
        return true;
    }
}

contract WineBar {
    @payer(addr)
    constructor(@space string f) {}

    function hello() public returns (bool) {
        return true;
    }
}

contract SpiritBar {
    @payer(addr)
    constructor(@other string f) {}

    function hello(@item uint32 c) public returns (bool) {
        return c==0;
    }
}

// ---- Expect: diagnostics ----
// error: 3:17-23: @payer annotation not allowed next to a parameter
// error: 13:34-40: duplicate @space annotation for constructor
// 	note 13:17-23: previous @space
// error: 22:17-23: conversion from string to uint64 not possible
// error: 31:17-23: unknown annotation other for constructor
// error: 33:20-25: parameter annotations are only allowed in constructors