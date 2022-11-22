contract hatchling {
    string name;

    constructor(string id) payable {
        require(id != "", "name must be provided");
        name = id;
    }
}

contract adult {
    function test() public {
        hatchling h = new hatchling{space: 10240}("luna");
    }
}
