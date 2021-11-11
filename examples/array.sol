// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract array {
	mapping (address => uint256) private _balances;
    uint256 private _totalSupply;

    function mint(address account, uint256 amount) public {
        _totalSupply += amount;

        _balances[account] = 0;
        _balances[account] += amount;
    }

    function balanceOf(address account) public view returns (uint256) {
        return _balances[account];
    }

    function totalSupply() public view returns (uint256) {
        return _totalSupply;
    }
}
