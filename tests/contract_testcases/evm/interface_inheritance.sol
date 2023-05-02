pragma solidity ^0.8.4;

interface IUniswapV2Router01 {
    function factory() external pure returns (address);
    function WETH() external pure returns (address);
    function createSomething(address tokenA, address tokenB) external returns (address);
}

interface IUniswapV2Router02 is IUniswapV2Router01 {
    function removeLiquidityETHSupportingFeeOnTransferTokens(
        address token,
        uint liquidity,
        uint amountTokenMin,
        uint amountETHMin,
        address to,
        uint deadline
    ) external returns (uint amountETH);
}

interface IUniswapV2Factory {
    function createPair(address tokenA, address tokenB) external returns (address);
}

contract BABYLINK { 

    IUniswapV2Router02 public uniswapV2Router;
    address public uniswapV2Pair;
        
    constructor () {
        
        IUniswapV2Router02 _uniswapV2Router = IUniswapV2Router02(0x10ED43C718714eb63d5aA57B78B54704E256024E); //Prod

        uniswapV2Router = _uniswapV2Router;
        
    }
    
    function changeRouterVersion(address newRouterAddress) public returns(address newPairAddress) {

        IUniswapV2Router02 _uniswapV2Router = IUniswapV2Router02(newRouterAddress); 


        if(newPairAddress == address(0)) //Create If Doesnt exist
        {
            newPairAddress = IUniswapV2Factory(_uniswapV2Router.factory())
                .createPair(address(this), _uniswapV2Router.WETH());
            uniswapV2Pair = _uniswapV2Router.createSomething({tokenA: address(this), tokenB:_uniswapV2Router.WETH()});
        }
    }
}
// ---- Expect: diagnostics ----
