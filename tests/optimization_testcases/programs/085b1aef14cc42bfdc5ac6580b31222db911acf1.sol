pragma solidity 0;

contract foo {
    struct s {
        int32 f1;
        bool f2;
    }

    function test() public {
        s[] bar = new s[](0);
        s memory n = bar.push();
        n.f1 = 102;
        n.f2 = true;

        assert(bar[0].f1 == 102);
        assert(bar[0].f2 == true);
    }
}
