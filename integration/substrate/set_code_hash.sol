import "substrate";

abstract contract Upgradeable {
    function set_code(Hash code) external {
        bytes _code = Hash.unwrap(code);
        require(set_code_hash(_code) == 0);
    }
}

contract SetCodeCounterV1 is Upgradeable {
    uint public count;

    function inc() external {
        count += 1;
    }
}

contract SetCodeCounterV2 is Upgradeable {
    uint public count;

    function inc() external {
        count += 1;
    }

    function dec() external {
        count -= 1;
    }
}
