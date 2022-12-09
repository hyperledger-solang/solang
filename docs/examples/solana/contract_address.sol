@program_id("5kQ3iJ43gHNDjqmSAtE1vDu18CiSAfNbRe4v5uoobh3U")
contract hatchling {
    string name;

    constructor(string id) payable {
        require(id != "", "name must be provided");
        name = id;
    }
}

contract adult {
    function test(address addr) public {
        hatchling h = new hatchling{address: addr}("luna");
    }
}
