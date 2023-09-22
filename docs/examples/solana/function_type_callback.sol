contract ft {
    function test(address p) public {
        // this.callback can be used as an external function type value
        paffling.set_callback{program_id: p}(this.callback);
    }

    function callback(int32 count, string foo) public {
        // ...
    }
}

contract paffling {
    // the first visibility "external" is for the function type, the second "internal" is
    // for the callback variables
    function(int32, string) external internal callback;

    function set_callback(function(int32, string) external c) public {
        callback = c;
    }

    function piffle() public {
        callback(1, "paffled");
    }
}
