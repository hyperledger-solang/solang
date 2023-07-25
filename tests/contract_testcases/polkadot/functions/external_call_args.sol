interface IUniswapV2Router01 {
    function WETH() external pure returns (address);

    function swapExactETHForTokens(
        uint amountOutMin,
        address[] calldata path,
        address to,
        uint deadline
    ) external payable returns (uint[] memory amounts);

    function swapTokensForExactETH(
        uint amountOut,
        uint amountInMax,
        address[] calldata path,
        address to,
        uint deadline
    ) external returns (uint[] memory amounts);

    function swapExactTokensForETH(
        uint amountIn,
        uint amountOutMin,
        address[] calldata path,
        address to,
        uint deadline
    ) external returns (uint[] memory amounts);

    function swapETHForExactTokens(
        uint amountOut,
        address[] calldata path,
        address to,
        uint deadline
    ) external payable returns (uint[] memory amounts);
}

library SafeMath {
    function mul(uint256 a, uint256 b) internal pure returns (uint256) {
        if (a == 0) return 0;
        uint256 c = a * b;
        require(c / a == b, "SafeMath: multiplication overflow");
        return c;
    }

    function div(uint256 a, uint256 b) internal pure returns (uint256) {
        require(b > 0, "SafeMath: division by zero");
        return a / b;
    }
}

contract Swap_ETH_TO_USDX {
    using SafeMath for uint;
    address distTokens;
    uint deadline;
    uint feerate;
    IUniswapV2Router01 public uniswapRouter;

    receive() external payable {
        address[] memory paths = new address[](2);
        paths[0] = uniswapRouter.WETH();
        paths[1] = distTokens;
        uint[] memory amounts = uniswapRouter.swapExactETHForTokens{
            value: msg.value.mul(feerate).div(10000)
        }({
            amountOutMin: 0,
            path: paths,
            to: address(this),
            deadline: block.timestamp + deadline
        });
    }
}
// ---- Expect: diagnostics ----
// warning: 60:23-30: local variable 'amounts' is unused
// warning: 61:20-53: conversion truncates uint256 to uint128, as value is type uint128 on target Polkadot
