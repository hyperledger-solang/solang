contract hatchling {
    string name;
    address private origin;

    constructor(string id, address parent) {
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
        hatchling h = new hatchling{salt: 0, gas: 10000}("luna", address(this));
    }
}
