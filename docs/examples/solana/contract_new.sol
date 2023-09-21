@program_id("5afzkvPkrshqu4onwBCsJccb1swrt4JdAjnpzK8N4BzZ")
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
    function test() external {
        hatchling.new("luna", address(this));
    }
}
