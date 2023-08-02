pragma solidity 0;

contract foo {
    struct s {
        int32 f1;
        bool f2;
    }

    function test() public {
        s[] bar = new s[](1);

        bar[0] = s({f1: 0, f2: false});
        bar.push(s({f1: 1, f2: true}));

        assert(bar.length == 2);
        assert(bar[1].f1 == 1);
        assert(bar[1].f2 == true);
    }
}
