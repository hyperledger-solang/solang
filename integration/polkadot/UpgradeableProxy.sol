// Integration test against the delegatecall() function in combination with input forwarding and tail call flags.
// WARNING: This code is neither EIP compliant nor secure nor audited nor intended to be used in production.

// SPDX-License-Identifier: MIT
// OpenZeppelin Contracts (last updated v4.6.0) (proxy/Proxy.sol)

/**
 * @dev This abstract contract provides a fallback function that delegates all calls to another contract using the EVM
 * instruction `delegatecall`. We refer to the second contract as the _implementation_ behind the proxy, and it has to
 * be specified by overriding the virtual {_implementation} function.
 *
 * Additionally, delegation to the implementation can be triggered manually through the {_fallback} function, or to a
 * different contract through the {_delegate} function.
 *
 * The success and return data of the delegated call will be returned back to the caller of the proxy.
 */
abstract contract Proxy {
    uint32 constant FORWARD_INPUT = 1;
    uint32 constant TAIL_CALL = 4;

    /**
     * @dev Delegates the current call to `implementation`.
     *
     * This function does not return to its internal call site. It will return directly to the external caller.
     */
    function _delegate(address implementation) internal virtual {
        implementation.delegatecall{flags: FORWARD_INPUT | TAIL_CALL}(hex"");
    }

    /**
     * @dev This is a virtual function that should be overridden so it returns the address to which the fallback function
     * and {_fallback} should delegate.
     */
    function _implementation() internal view virtual returns (address);

    /**
     * @dev Delegates the current call to the address returned by `_implementation()`.
     *
     * This function does not return to its internal call site. It will return directly to the external caller.
     */
    function _fallback() internal virtual {
        _beforeFallback();
        _delegate(_implementation());
    }

    /**
     * @dev Fallback function that delegates calls to the address returned by `_implementation()`. It will run if no other
     * function in the contract matches the call data.
     */
    fallback() external virtual {
        _fallback();
    }

    /**
     * @dev Fallback function that delegates calls to the address returned by `_implementation()`. It will run if call data
     * is empty.
     */
    receive() external payable virtual {
        _fallback();
    }

    /**
     * @dev Hook that is called before falling back to the implementation. Can happen as part of a manual `_fallback`
     * call, or as part of the Solidity `fallback` or `receive` functions.
     *
     * If overridden should call `super._beforeFallback()`.
     */
    function _beforeFallback() internal virtual {}
}

// FIXME: This is NOT EIP-1967.
// Have to mock it this way until issues #1387 and #1388 are resolved.
abstract contract StorageSlot {
    mapping(bytes32 => address) getAddressSlot;
}

// Minimal proxy implementation; without security
contract UpgradeableProxy is Proxy, StorageSlot {
    event Upgraded(address indexed implementation);

    bytes32 internal constant IMPLEMENTATION_SLOT =
        0x360894a13ba1a3210667c828492db98dca3e2076cc3735a920a3ca505d382bbc;

    function _setImplementation(address newImplementation) private {
        // FIXME once issue #809 (supporting address.code) is solved
        // if (newImplementation.code.length == 0) {
        //     revert ERC1967InvalidImplementation(newImplementation);
        // }
        // FIXME see #1387 and #1388
        getAddressSlot[IMPLEMENTATION_SLOT] = newImplementation;
    }

    function upgradeTo(address newImplementation) public {
        _setImplementation(newImplementation);
        emit Upgraded(newImplementation);
    }

    function upgradeToAndCall(
        address newImplementation,
        bytes memory data
    ) public returns (bytes ret) {
        upgradeTo(newImplementation);
        (bool ok, ret) = newImplementation.delegatecall(data);
        require(ok);
    }

    function _implementation()
        internal
        view
        virtual
        override
        returns (address)
    {
        return getAddressSlot[IMPLEMENTATION_SLOT];
    }
}

// Proxy implementation v1
contract UpgradeableImplV1 {
    uint public count;

    function inc() external {
        print("v1");
        count += 1;
    }
}

// Proxy implementation v2
contract UpgradeableImplV2 {
    uint public count;
    string public version;

    function setVersion() public {
        version = "v2";
    }

    function inc() external {
        count += 1;
    }

    function dec() external {
        count -= 1;
    }
}
