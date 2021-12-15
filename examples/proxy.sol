abstract contract Storage {
    uint256 public value;

    address public implementation;
}

contract Implementation is Storage {
    function setValue(uint256 v) public {
        value = v;
    }

    function getValue() public returns (uint256) {
        return value;
    }
}

contract Proxy is Storage {

    function setImplemention(address _impl) public {
        implementation = _impl;
    }

    fallback() external {
        (, bytes memory result) = address(implementation).delegatecall(msg.data);

        return2(result);
    }
}

contract Test {

    address public proxy;

    function setProxy(address _proxy) public {
        proxy = _proxy;
    }

    function testSet() public {
        Implementation(proxy).setValue(5);
    }

    function testGet() public returns (uint256) {
        return Implementation(proxy).getValue();
    }
}
