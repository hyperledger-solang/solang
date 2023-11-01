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
        string calldata _payment,
	uint64 value
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
            total == value,
            "Total payment value does not match ether sent"
        );

        for (uint8 i = 0; i < _receivers.length; i++) {
            payable(_receivers[i]).transfer(_amounts[i]);
        }

        emit Receipt(
            address(this),
            address"11111111111111111111111111111111",
            _receivers,
            _amounts,
            _payment
        );
    }
}


// ---- Expect: diagnostics ----
// error: 34:13-57: method 'transfer' not available on Solana. Use the lamports field from the AccountInfo struct directly to operate on balances.
