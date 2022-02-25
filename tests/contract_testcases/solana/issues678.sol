// SPDX-License-Identifier: MIT
pragma solidity ^0.7.0;

contract Shares {
    event Transfer(address to, uint amount, uint balance);

    struct Share {
        address payable shareOwner;
        uint amount;
    }

    Share[] private _shares;

    /// @notice Create the shares object that gives the shares to every body
    constructor() {
        _shares.push(Share(msg.sender, 1000));
    }

    function getShares() external view returns(address[] memory, uint[] memory) {
        address[] memory retAddress = new address[](uint32(_shares.length));
        uint[] memory retShare = new uint[](uint32(_shares.length));
        for (uint i = 0; i < _shares.length; i++) {
            retAddress[i] = _shares[i].shareOwner;
            retShare[i] = _shares[i].amount;
        }
        return (retAddress, retShare);
    }

    function _senderIsAShareOwner() private view returns (bool) {
        for (uint i = 0; i < _shares.length; i++) {
            if (_shares[i].shareOwner == msg.sender) {
                return true;
            }
        }
        return false;
    }

    /**
    ** @dev Allow a share owner to retrieve his money. It empty the money contained inside of the smart contract to give it to owners.
     */
    function withdraw() external {
        require(_senderIsAShareOwner(), "You can't withdraw if you are not a share owner");
        uint curr_balance = address(this).balance;
        require(curr_balance > 0, "There is nothing to withdraw");
        for (uint i = 0; i < _shares.length; i++) {
            uint to_transfer = curr_balance * _shares[i].amount / 1000;
            _shares[i].shareOwner.transfer(uint64(to_transfer));
            emit Transfer(_shares[i].shareOwner, to_transfer, curr_balance);
        }
        if (address(this).balance > 0) {
            // Send the remaining money to the one who withdraw so there is nothing left on
            // the contract
            payable(msg.sender).transfer(address(this).balance);
        }
    }
}
