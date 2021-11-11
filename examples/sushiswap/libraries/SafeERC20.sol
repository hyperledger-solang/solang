// SPDX-License-Identifier: MIT
pragma solidity 0.6.12;

import "../interfaces/IERC20.sol";

library SafeERC20 {
    function safeSymbol(IERC20 token) internal returns(string memory) {
        (bool success, bytes memory data) = address(token).staticcall(abi.encodeWithSelector(0x95d89b41));
        if (success && data.length > 0) {
            return abi.decode(data, (string));
        }
        
        return "???";
    }

    function safeName(IERC20 token) internal returns(string memory) {
        (bool success, bytes memory data) = address(token).staticcall(abi.encodeWithSelector(0x06fdde03));
        if (success && data.length > 0) {
            return abi.decode(data, (string));
        }

        return "???";
    }

    function safeDecimals(IERC20 token) public returns (uint8) {
        (bool success, bytes memory data) = address(token).staticcall(abi.encodeWithSelector(0x313ce567));
        return success && data.length == 32 ? abi.decode(data, (uint8)) : 18;
    }

    function safeTransfer(IERC20 token, address to, uint256 amount) internal {
        (bool success, bytes memory data) = address(token).call(abi.encodeWithSelector(0xa9059cbb, to, amount));
        require(success && (data.length == 0 || abi.decode(data, (bool))), "SafeERC20: Transfer failed");
    }

    function safeTransferFrom(IERC20 token, address from, uint256 amount) internal {
        (bool success, bytes memory data) = address(token).call(abi.encodeWithSelector(0x23b872dd, from, address(this), amount));
        require(success && (data.length == 0 || abi.decode(data, (bool))), "SafeERC20: TransferFrom failed");
    }
}
