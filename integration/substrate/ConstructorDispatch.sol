contract ConstructorDispatch {
    address admin;

    constructor() {
        admin = msg.sender;
    }

    function boss() public view returns (address) {
        return admin;
    }
}

contract HappyCaller {
    function call(address callee, bytes input) public {
        callee.call(input);
    }
}
