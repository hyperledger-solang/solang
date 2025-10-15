// SPDX-License-Identifier: UNLICENSED
pragma solidity >=0.8.0;

interface ArbWasm {
    /// @notice Activate a wasm program
    /// @param program the program to activate
    /// @return version the stylus version the program was activated against
    /// @return dataFee the data fee paid to store the activated program
    function activateProgram(
        address program
    ) external payable returns (uint16 version, uint256 dataFee);
}

contract C is ArbWasm {
    function forwardActivateProgram(
        address target,
        address program
    ) public payable returns (uint16, uint256) {
        print("target = {}".format(target));
        (uint16 version, uint256 dataFee) = ArbWasm(target).activateProgram{
            value: msg.value
        }(program);
        print("version = {}".format(version));
        print("dataFee = {}".format(dataFee));
        return (version, dataFee);
    }

    function activateProgram(
        address program
    ) public payable returns (uint16, uint256) {
        print("program = {}".format(program));
        uint256 value = msg.value;
        print("value = {}".format(value));
        return (1, 1000);
    }
}

contract D {
    function greet() public pure {
        print("Hello!");
    }
}
