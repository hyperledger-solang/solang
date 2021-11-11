// SPDX-License-Identifier: GPL-2.0-or-later
pragma solidity =0.7.6;
pragma abicoder v2;

import '../uniswapV3core/interfaces/IUniswapV3Pool.sol';
import '../uniswapV3core/libraries/SafeERC20Namer.sol';

import './libraries/ChainId.sol';
import './interfaces/INonfungiblePositionManager.sol';
import './interfaces/INonfungibleTokenPositionDescriptor.sol';
import './interfaces/IERC20Metadata.sol';
import './libraries/PoolAddress.sol';
import './libraries/NFTDescriptor.sol';
import './libraries/TokenRatioSortOrder.sol';

/// @title Describes NFT token positions
/// @notice Produces a string containing the data URI for a JSON metadata string
contract NonfungibleTokenPositionDescriptor is INonfungibleTokenPositionDescriptor {
    address private constant DAI = address(0x6b175474e89094c44da98b954eedeac495271d0f);
    address private constant USDC = address(0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48);
    address private constant USDT = address(0xdac17f958d2ee523a2206206994597c13d831ec7);
    address private constant TBTC = address(0x8daebade922df735c38c80c7ebd708af50815faa);
    address private constant WBTC = address(0x2260fac5e5542a773aa44fbcfedf7c193bc2c599);

    int256 constant NUMERATOR_MOST = 300;
    int256 constant NUMERATOR_MORE = 200;
    int256 constant NUMERATOR = 100;

    int256 constant DENOMINATOR_MOST = -300;
    int256 constant DENOMINATOR_MORE = -200;
    int256 constant DENOMINATOR = -100;

    address public immutable WETH9;

    constructor(address _WETH9) {
        WETH9 = _WETH9;
    }

    /// @inheritdoc INonfungibleTokenPositionDescriptor
    function tokenURI(INonfungiblePositionManager positionManager, uint256 tokenId)
        external
        override
        returns (string memory)
    {
        (, , address token0, address token1, uint24 fee, int24 tickLower, int24 tickUpper, , , , , ) =
            positionManager.positions(tokenId);

        IUniswapV3Pool pool =
            IUniswapV3Pool(
                PoolAddress.computeAddress(
                    positionManager.factory(),
                    PoolAddress.PoolKey({token0: token0, token1: token1, fee: fee})
                )
            );

        bool _flipRatio = flipRatio(token0, token1, ChainId.get());
        address quoteTokenAddress = !_flipRatio ? token1 : token0;
        address baseTokenAddress = !_flipRatio ? token0 : token1;
        (, int24 tick, , , , , ) = pool.slot0();

        return
            NFTDescriptor.constructTokenURI(
                NFTDescriptor.ConstructTokenURIParams({
                    tokenId: tokenId,
                    quoteTokenAddress: quoteTokenAddress,
                    baseTokenAddress: baseTokenAddress,
                    quoteTokenSymbol: quoteTokenAddress == WETH9
                        ? string('ETH')
                        : SafeERC20Namer.tokenSymbol(quoteTokenAddress),
                    baseTokenSymbol: baseTokenAddress == WETH9 ? string('ETH') : SafeERC20Namer.tokenSymbol(baseTokenAddress),
                    quoteTokenDecimals: IERC20Metadata(quoteTokenAddress).decimals(),
                    baseTokenDecimals: IERC20Metadata(baseTokenAddress).decimals(),
                    flipRatio: _flipRatio,
                    tickLower: tickLower,
                    tickUpper: tickUpper,
                    tickCurrent: tick,
                    tickSpacing: pool.tickSpacing(),
                    fee: fee,
                    poolAddress: address(pool)
                })
            );
    }

    function flipRatio(
        address token0,
        address token1,
        uint256 chainId
    ) public view returns (bool) {
        return tokenRatioPriority(token0, chainId) > tokenRatioPriority(token1, chainId);
    }

    function tokenRatioPriority(address token, uint256 chainId) public view returns (int256) {
        if (token == WETH9) {
            return DENOMINATOR;
        }
        if (chainId == 1) {
            if (token == USDC) {
                return NUMERATOR_MOST;
            } else if (token == USDT) {
                return NUMERATOR_MORE;
            } else if (token == DAI) {
                return NUMERATOR;
            } else if (token == TBTC) {
                return DENOMINATOR_MORE;
            } else if (token == WBTC) {
                return DENOMINATOR_MOST;
            } else {
                return 0;
            }
        }
        return 0;
    }
}
