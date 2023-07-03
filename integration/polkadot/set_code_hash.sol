import "polkadot";

abstract contract Upgradeable {
    function set_code(uint8[32] code) external {
        require(set_code_hash(code) == 0);
    }
}

contract SetCodeCounterV1 is Upgradeable {
    uint public count;

    constructor(uint _count) {
        count = _count;
    }

    function inc() external {
        count += 1;
    }
}

contract SetCodeCounterV2 is Upgradeable {
    uint public count;

    constructor(uint _count) {
        count = _count;
    }

    function inc() external {
        count -= 1;
    }
}
