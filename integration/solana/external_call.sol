
contract caller {
    
    @account(callee_pid)
    function do_call(int64 v) external {
        callee.set_x{program_id: tx.accounts.callee_pid.key}(v);
    }

    @account(callee_pid)
    function do_call2(int64 v) view external returns (int64) {
        return v + callee.get_x{program_id: tx.accounts.callee_pid.key}();
    }

    // call two different functions
    @account(callee_pid)
    @account(callee2_pid)
    function do_call3(int64[4] memory x, string memory y) external returns (int64, string memory) {
        return (callee2.do_stuff{program_id: tx.accounts.callee2_pid.key}(x), 
                callee.get_name{program_id: tx.accounts.callee_pid.key}());
    }

    // call two different functions
    @account(callee_pid)
    @account(callee2_pid)
    function do_call4(int64[4] memory x, string memory y) external returns (int64, string memory) {
        return (callee2.do_stuff{program_id: tx.accounts.callee2_pid.key}(x), 
                callee.call2{program_id: tx.accounts.callee_pid.key}(y));
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

    @account(other_callee2)
    function call2(string s) external returns (string) {
        return callee2.do_stuff2{program_id: tx.accounts.other_callee2.key}(s);
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