contract hatchling {
    string name;
    address private origin;

    constructor(string id, address parent) payable {
        require(id != "", "name must be provided");
        name = id;
        origin = parent;
    }

    function root() public returns (address) {
        return origin;
    }
}

contract adult {
    function test() public {
        hatchling h = new hatchling{value: 500}("luna", address(this));
    }
}
