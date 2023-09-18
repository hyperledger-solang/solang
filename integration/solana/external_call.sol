
contract caller {
    function do_call(address e, int64 v) public {
        callee.set_x{program_id: e}(v);
    }

    function do_call2(address e, int64 v) view public returns (int64) {
        return v + callee.get_x{program_id: e}();
    }

    // call two different functions
    function do_call3(address e, address e2, int64[4] memory x, string memory y) pure public returns (int64, string memory) {
        return (callee2.do_stuff{program_id: e2}(x), callee.get_name{program_id: e}());
    }

    // call two different functions
    function do_call4(address e, address e2, int64[4] memory x, string memory y) pure public returns (int64, string memory) {
        return (callee2.do_stuff{program_id: e2}(x), callee.call2{program_id: e}(e2, y));
    }

    function who_am_i() public view returns (address) {
        return address(this);
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

    function call2(address e2, string s) public pure returns (string) {
        return callee2.do_stuff2{program_id: e2}(s);
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