function free_standing(@item string a) returns (uint32) {
    return a.length;
}

function free_standing2(string a) returns (@ret uint32 ret) {
    ret = a.length;
}


contract MyTest {
    constructor() {}

    modifier MyMod(@item1 string c) {
        _;
    }

    function test1(@ann bytes one) public pure returns (uint32) {
        return one.length;
    }

    function test2(bytes one) public pure returns (@um uint32 item, @hm uint256) {
        return (one.length, 36);
    }

    function test3() external pure returns (uint32, int8) {
        return (34, 21);
    }

    function test4() internal pure returns (uint32, int8) {
        try this.test3() returns (@item uint32 a, int8 b) {
            return (a, b);
        } catch (@item2 bytes err) {
            return (0,0);
        }
    }

    function test5() internal pure returns (uint32, int8) {
        try this.test3() returns (uint32 a, int8 b) {
            return (a, b);
        } catch (@item2 bytes err) {
            return (0,0);
        }
    }

    function test6() internal pure returns (uint32, int8) {
        try this.test3() returns (uint32 a, int8 b) {
            return (a, b);
        } catch Error (@item2 string err) {
            return (0,0);
        }
    }

    function test7() public pure returns (uint32) {
        function(@item1 bytes bb) returns (int32) fPtr = test1;
        return fPtr("bc");
    }

    function test8() public pure returns (uint32, int8) {
        function() external returns (@smt uint32, @utm int8) fPtr = this.test3;
        return fPtr();
    }
}

// ---- Expect: diagnostics ----
// error: 1:24-29: unexpected parameter annotation
// error: 5:44-48: unexpected parameter annotation
// error: 13:20-26: unexpected parameter annotation
// error: 17:20-24: unexpected parameter annotation
// error: 21:52-55: unexpected parameter annotation
// error: 21:69-72: unexpected parameter annotation
// error: 30:35-40: unexpected parameter annotation
// error: 40:18-24: unexpected parameter annotation
// error: 48:24-30: unexpected parameter annotation
// error: 54:18-24: unexpected parameter annotation
// error: 59:38-42: unexpected parameter annotation
// error: 59:51-55: unexpected parameter annotation