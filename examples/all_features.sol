// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract AllFeatures {
    uint public value;

    function testEcrecover() public pure returns (address) {
        return ecrecover(bytes32(hex"0000000000000000000000000000000000000000000000000000000000000049"), uint8(0), bytes32(hex"000000000000000000000000000000000000000000000000000000000000004C"), bytes32(hex"0000000000000000000000000000000000000000000000000000000000000055"));
    }

    function testMulmod() public pure returns (uint256)
    {
        return mulmod(0x000000000000000000000000000000000000000000000000000000000000000a, 0x00000000000000000000000000000000000000000000000000000000000f3688,  ~uint256(0));
    }

    function testHash() public pure returns (bytes20) {
        return ripemd160("ss");
    }

    function testMsgSender() public view returns (address) {
        return msg.sender;
    }

    function testBalance() public payable returns (uint256) {
        value = address(this).balance;

        return address(this).balance;
    }

    function testMsgValue() public payable returns (uint256) {
        value = msg.value;

        return msg.value;
    }

    function testMsgSig() public pure returns (bytes4) {
        return msg.sig;
    }
    
    function testGas() public view returns (uint256) {
        return gasleft();
    }

    function testTxGasprice() public view returns (uint256) {
        return tx.gasprice;
    }

    function testTxOrigin() public view returns (address) {
        return tx.origin;
    }

    function testBlockChainId() public pure returns (uint) {
        return block.chainid;
    }

    function testBlockCoinbase() public view returns (address) {
        return block.coinbase;
    }

    function testBlockGaslimit () public view returns (uint256) {
        return block.gaslimit;
    }

    function testBlockNumber () public view returns (uint256) {
        return block.number;
    }

    function testBlockDifficulty() public view returns (uint256) {
        return block.difficulty;
    }

    function testBalance(address a) public pure returns (uint256) {
        return a.balance;
    }

    function testBlockTimestampy() public view returns (uint256) {
        return block.timestamp;
    }
}
