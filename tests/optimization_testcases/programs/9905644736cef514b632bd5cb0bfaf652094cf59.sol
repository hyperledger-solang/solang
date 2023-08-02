pragma solidity 0;

contract foo {
    struct s {
        int32 f1;
        bool f2;
    }

    function test() public {
        s[] bar = new s[](1);

        bar[0] = s(128, true);

        assert(bar.length == 1);

        s baz = bar.pop();
        assert(baz.f1 == 128);
        assert(baz.f2 == true);
        assert(bar.length == 0);
    }
}
