
contract caller {
    function do_call(callee e, int64 v) public {
        e.set_x(v);
    }

    function do_call2(callee e, int64 v) view public returns (int64) {
        return v + e.get_x();
    }

    // call two different functions
    function do_call3(callee e, callee2 e2, int64[4] memory x, string memory y) pure public returns (int64, string memory) {
        return (e2.do_stuff(x), e.get_name());
    }

    // call two different functions
    function do_call4(callee e, callee2 e2, int64[4] memory x, string memory y) public returns (int64, string memory) {
        return (e2.do_stuff(x), e.call2(e2, y));
    }
}

contract callee {
    int64 x;

    function set_x(int64 v) public {
        x = v;
    }

    function get_x() public view returns (int64) {
        return x;
    }

    function call2(callee2 e2, string s) public returns (string) {
        return e2.do_stuff2(s);
    }

    function get_name() public pure returns (string) {
        return "my name is callee";
    }
}

contract callee2 {
    function do_stuff(int64[4] memory x) public pure returns (int64) {
        int64 total = 0;

        for (uint i=0; i< x.length; i++)  {
            total += x[i];
        }

        return total;
    }

    function do_stuff2(string x) public pure returns (string) {
        return "x:" + x;
    }
}