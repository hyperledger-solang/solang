// SPDX-License-Identifier: UNLICENSED
pragma solidity >=0.8.0;

contract C {
    function test() public view returns (uint256, bytes32, bytes32, uint256) {
        bytes memory code = address(this).code;

        uint256 balance = address(this).balance;
        bytes32 codehash = address(this).codehash;
        bytes32 manual_codehash = keccak256(code);
        uint256 gasprice = tx.gasprice;

        print("balance = {}".format(balance));
        print("codehash = {}".format(codehash));
        print("manual_codehash = {}".format(manual_codehash));
        print("gasprice = {}".format(gasprice));

        assert(codehash == manual_codehash);

        return (balance, codehash, manual_codehash, gasprice);
    }

    function test_addmod() public pure {
        uint256 x = addmod(500, 100, 3);
        uint256 y = addmod(500, 100, 7);

        print("x = {}".format(x));
        print("y = {}".format(y));

        assert(x == 0);
        assert(y == 5);
    }

    function test_mulmod() public pure {
        uint256 x = mulmod(500, 100, 3);
        uint256 y = mulmod(500, 100, 7);

        print("x = {}".format(x));
        print("y = {}".format(y));

        assert(x == 2);
        assert(y == 6);
    }

    function getCode() public view returns (bytes) {
        return address(this).code;
    }

    function accept_donation() public payable {}
}
