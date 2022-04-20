// SPDX-License-Identifier: MIT
pragma solidity >=0.8.4;

contract MyContract {
    event Receipt(
        address From,
                address Token,
        address[] Receivers,
        uint64[] Amounts,
        string Payment
    );

    function send( 
        address[] calldata _receivers,
        uint64[] calldata _amounts,
        string calldata _payment
    ) external payable {
        require(
            _receivers.length == _amounts.length,
            "Receiver count does not match amount count."
        );

        uint64 total = 0;
        for (uint8 i = 0; i < _receivers.length; i++) {
            total += _amounts[i];
        }
        require(
            total == msg.value,
            "Total payment value does not match ether sent"
        );

        for (uint8 i = 0; i < _receivers.length; i++) {
            (bool sent, ) = _receivers[i].call{value: _amounts[i]}("");
            require(sent, "Transfer failed.");
        }

        emit Receipt(
            msg.sender,
            address"11111111111111111111111111111111",
            _receivers,
            _amounts,
            _payment
        );
    }
}

