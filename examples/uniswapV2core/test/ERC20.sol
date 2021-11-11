pragma solidity =0.5.16;

import '../UniswapV2ERC20.sol';

contract ERC20 is UniswapV2ERC20 {
    function initERC20(uint _totalSupply) public {
        initUniswapV2ERC20();

        _mint(msg.sender, _totalSupply);
    }
}
