contract hatchling {
    string name;

    constructor(string id) payable {
        require(id != "", "name must be provided");
        name = id;
    }
}

contract adult {
    function test(address id) external {
        hatchling.new{program_id: id}("luna");
    }
}
