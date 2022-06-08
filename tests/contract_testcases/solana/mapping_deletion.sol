contract savedTest {
    uint b;
    constructor(uint d) {
        b = d;
    }

    function increment(uint f) public {
        b += f;
    }
}

contract DeleteTest {
    struct data_struct  {
        address addr1;
	    address addr2;
    }

    mapping(uint => data_struct) example;
    mapping(uint => savedTest) example2;

    function addData() public  {
        data_struct dt = data_struct({addr1: address(this), addr2: msg.sender});
        uint id = 1;
        example[id] = dt;
        savedTest tt = new savedTest(4);
        example2[id] = tt;
    }

    function deltest() external {
        uint id = 1;
        delete example[id];
        //delete example2[id];
    }
    
    function get() public view returns (data_struct calldata) {
        uint id = 1;
        return example[id];
    }

}