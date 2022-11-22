contract test {
    using lib for int32[100];

    int32[100] bar;

    function foo() public {
        bar.set(10, 571);
    }
}

library lib {
    function set(
        int32[100] storage a,
        uint256 index,
        int32 val
    ) internal {
        a[index] = val;
    }
}
