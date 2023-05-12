contract foo {
    function bar() public pure returns (int, int) {
        return (1, 2);
    }
}

contract BABYLINK { 
    struct tts{
        int a;
        int b;
    }

    function multipleRetuns(int c) public pure returns (int, int, int) {
        return (1, c + 2, 3);
    }

    function singleReturn() private pure returns (uint) {
        return 3;
    }

    function returnBool() private pure returns (bool) {
        return true;
    }
    
    function testing() public returns (int) {
        multipleRetuns(3); 
        int b = 5;
        multipleRetuns({c: 9});
        1 + singleReturn();
        1 - singleReturn();
        1 * singleReturn();
        1 / singleReturn();
        1 | singleReturn();
        1 & singleReturn();
        1 << singleReturn();
        1 >> singleReturn();
        !returnBool();
        ~singleReturn();
        +singleReturn();
        -singleReturn();
        foo r = new foo();
        r.bar();

        1 + (1.3 + 1.8);

        function (int) external returns (int, int, int) fptr = this.multipleRetuns;
        function (int) internal returns (int, int, int) fptr2 = multipleRetuns;

        fptr(3);
        fptr2(3);

        address(this).call("multipleRetuns");
        tts(1, 2);
        tts({a: 1, b:2});

        return b;
    }
}
// ---- Expect: diagnostics ----
// error: 39:9-24: unary plus not permitted
// error: 40:9-24: negate not allowed on unsigned
