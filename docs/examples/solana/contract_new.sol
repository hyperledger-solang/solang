@program_id("8scvhNyoxUUo7e3hRnUzcTtFtcZ3LdXHuy8b42Hd5d2T")
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

contract creator {
    function create_hatchling(address new_address) public {
        hatchling h;
       
	h = new hatchling{address: new_address}("luna", address(this));
    }
}
