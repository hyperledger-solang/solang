// SPDX-License-Identifier: BUSL-1.1
pragma solidity =0.7.6;

import './interfaces/IUniswapV3Pool.sol';

import './NoDelegateCall.sol';

import './libraries/LowGasSafeMath.sol';
import './libraries/SafeCast.sol';
import './libraries/Tick.sol';
import './libraries/TickBitmap.sol';
import './libraries/Position.sol';

import './libraries/FixedPoint128.sol';
import './libraries/TransferHelper.sol';
import './libraries/LiquidityMath.sol';
import './libraries/SqrtPriceMath.sol';
import './libraries/Oracle.sol';

import './libraries/ITickMath.sol';
import './libraries/ISwapMath.sol';
import './libraries/IFullMath.sol';

import './interfaces/IUniswapV3PoolDeployer.sol';
import './interfaces/IUniswapV3Factory.sol';

contract UniswapV3Pool is IUniswapV3Pool, NoDelegateCall {
    using LowGasSafeMath for uint256;
    using LowGasSafeMath for int256;
    using SafeCast for uint256;
    using SafeCast for int256;
    using Tick for mapping(int24 => Tick.Info);
    using TickBitmap for mapping(int16 => uint256);
    using Position for mapping(bytes32 => Position.Info);
    using Position for Position.Info;
    using Oracle for Oracle.Observation[65535];

    uint256 internal constant Q128 = 0x100000000000000000000000000000000;

    int24 public constant MIN_TICK = -887272;
    int24 public constant MAX_TICK = -MIN_TICK;
    uint160 public constant MIN_SQRT_RATIO = 4295128739;
    uint160 public constant MAX_SQRT_RATIO = 1461446703485210103287273052203988822378723970342;

    address internal constant TickMath = address(0xb8927ee31817410e75093841d32a16bba69c0b4d);
    address internal constant SwapMath = address(0x3f921ca425721442da07037bdd3b8b8fb3f3dcb2);
    address internal constant FullMath = address(0x147b241f1aca14abcb927e8d5b5225cfbbb9d232);

    address internal constant UniswapV3PoolActions = address(0x1edf67678c5a5dce3d1ffe1857f237cc9459030);

    // @inheritdoc IUniswapV3PoolImmutables
    address public override factory;
    // @inheritdoc IUniswapV3PoolImmutables
    address public override token0;
    // @inheritdoc IUniswapV3PoolImmutables
    address public override token1;
    // @inheritdoc IUniswapV3PoolImmutables
    uint24 public override fee;

    // @inheritdoc IUniswapV3PoolImmutables
    int24 public override tickSpacing;

    // @inheritdoc IUniswapV3PoolImmutables
    uint128 public override maxLiquidityPerTick;

    struct Slot0 {
        // the current price
        uint160 sqrtPriceX96;
        // the current tick
        int24 tick;
        // the most-recently updated index of the observations array
        uint16 observationIndex;
        // the current maximum number of observations that are being stored
        uint16 observationCardinality;
        // the next maximum number of observations to store, triggered in observations.write
        uint16 observationCardinalityNext;
        // the current protocol fee as a percentage of the swap fee taken on withdrawal
        // represented as an integer denominator (1/x)%
        uint8 feeProtocol;
        // whether the pool is locked
        bool unlocked;
    }
    // @inheritdoc IUniswapV3PoolState
    Slot0 public override slot0;

    // @inheritdoc IUniswapV3PoolState
    uint256 public override feeGrowthGlobal0X128;
    // @inheritdoc IUniswapV3PoolState
    uint256 public override feeGrowthGlobal1X128;

    // accumulated protocol fees in token0/token1 units
    struct ProtocolFees {
        uint128 token0;
        uint128 token1;
    }
    // @inheritdoc IUniswapV3PoolState
    ProtocolFees public override protocolFees;

    // @inheritdoc IUniswapV3PoolState
    uint128 public override liquidity;

    // @inheritdoc IUniswapV3PoolState
    mapping(int24 => Tick.Info) public override ticks;
    // @inheritdoc IUniswapV3PoolState
    mapping(int16 => uint256) public override tickBitmap;
    // @inheritdoc IUniswapV3PoolState
    mapping(bytes32 => Position.Info) public override positions;
    // @inheritdoc IUniswapV3PoolState
    Oracle.Observation[65535] public override observations;

    /// @dev Mutually exclusive reentrancy protection into the pool to/from a method. This method also prevents entrance
    /// to a function before the pool is initialized. The reentrancy guard is required throughout the contract because
    /// we use balance checks to determine the payment status of interactions such as mint, swap and flash.
    modifier lock() {
        require(slot0.unlocked, 'LOK');
        slot0.unlocked = false;
        _;
        slot0.unlocked = true;
    }

    /// @dev Prevents calling a function from anyone except the address returned by IUniswapV3Factory#owner()
    modifier onlyFactoryOwner() {
        require(msg.sender == IUniswapV3Factory(factory).owner());
        _;
    }

    function init() external {
        initNoDelegateCall();

        (address _factory, address _token0, address _token1, uint24 _fee, int24 _tickSpacing) = IUniswapV3PoolDeployer(msg.sender).parameters();
        
        tickSpacing = _tickSpacing;
        factory = _factory;
        token0 = _token0;
        token1 = _token1;
        fee = _fee;

        maxLiquidityPerTick = Tick.tickSpacingToMaxLiquidityPerTick(_tickSpacing);
    }

    /// @dev Common checks for valid tick inputs.
    function checkTicks(int24 tickLower, int24 tickUpper) private {
        require(tickLower < tickUpper, 'TLU');
        require(tickLower >= MIN_TICK, 'TLM');
        require(tickUpper <= MAX_TICK, 'TUM');
    }

    /// @dev Returns the block timestamp truncated to 32 bits, i.e. mod 2**32. This method is overridden in tests.
    function _blockTimestamp() internal view virtual returns (uint32) {
        return uint32(block.timestamp); // truncation is desired
    }

    /// @dev Get the pool's balance of token0
    /// @dev This function is gas optimized to avoid a redundant extcodesize check in addition to the returndatasize
    /// check
    function balance0() public returns (uint256) {
        (bool success, bytes memory data) =
            address(token0).staticcall(abi.encodeWithSignature('balanceOf(address)', address(this)));
        require(success && data.length >= 32);
        return abi.decode(data, (uint256));
    }

    /// @dev Get the pool's balance of token1
    /// @dev This function is gas optimized to avoid a redundant extcodesize check in addition to the returndatasize
    /// check
    function balance1() public returns (uint256) {
        (bool success, bytes memory data) =
            address(token1).staticcall(abi.encodeWithSignature('balanceOf(address)', address(this)));
        require(success && data.length >= 32);
        return abi.decode(data, (uint256));
    }

    // @inheritdoc IUniswapV3PoolDerivedState
    function snapshotCumulativesInside(int24 tickLower, int24 tickUpper)
        external
        override
        noDelegateCall
        returns (
            int56 tickCumulativeInside,
            uint160 secondsPerLiquidityInsideX128,
            uint32 secondsInside
        )
    {
        checkTicks(tickLower, tickUpper);

        int56 tickCumulativeLower;
        int56 tickCumulativeUpper;
        uint160 secondsPerLiquidityOutsideLowerX128;
        uint160 secondsPerLiquidityOutsideUpperX128;
        uint32 secondsOutsideLower;
        uint32 secondsOutsideUpper;

        {
            Tick.Info storage lower = ticks[tickLower];
            Tick.Info storage upper = ticks[tickUpper];
            bool initializedLower;
            (tickCumulativeLower, secondsPerLiquidityOutsideLowerX128, secondsOutsideLower, initializedLower) = (
                lower.tickCumulativeOutside,
                lower.secondsPerLiquidityOutsideX128,
                lower.secondsOutside,
                lower.initialized
            );
            require(initializedLower);

            bool initializedUpper;
            (tickCumulativeUpper, secondsPerLiquidityOutsideUpperX128, secondsOutsideUpper, initializedUpper) = (
                upper.tickCumulativeOutside,
                upper.secondsPerLiquidityOutsideX128,
                upper.secondsOutside,
                upper.initialized
            );
            require(initializedUpper);
        }

        Slot0 memory _slot0 = slot0;

        if (_slot0.tick < tickLower) {
            return (
                tickCumulativeLower - tickCumulativeUpper,
                secondsPerLiquidityOutsideLowerX128 - secondsPerLiquidityOutsideUpperX128,
                secondsOutsideLower - secondsOutsideUpper
            );
        } else if (_slot0.tick < tickUpper) {
            uint32 time = _blockTimestamp();
            (int56 tickCumulative, uint160 secondsPerLiquidityCumulativeX128) =
                observations.observeSingle(
                    time,
                    0,
                    _slot0.tick,
                    _slot0.observationIndex,
                    liquidity,
                    _slot0.observationCardinality
                );
            return (
                tickCumulative - tickCumulativeLower - tickCumulativeUpper,
                secondsPerLiquidityCumulativeX128 -
                    secondsPerLiquidityOutsideLowerX128 -
                    secondsPerLiquidityOutsideUpperX128,
                time - secondsOutsideLower - secondsOutsideUpper
            );
        } else {
            return (
                tickCumulativeUpper - tickCumulativeLower,
                secondsPerLiquidityOutsideUpperX128 - secondsPerLiquidityOutsideLowerX128,
                secondsOutsideUpper - secondsOutsideLower
            );
        }
    }
    
    // @inheritdoc IUniswapV3PoolDerivedState
    function observe(bytes calldata secondsAgos)
        external
        view
        override
        noDelegateCall
        returns (int56[] memory tickCumulatives, uint160[] memory secondsPerLiquidityCumulativeX128s)
    {
        (tickCumulatives, secondsPerLiquidityCumulativeX128s) = observations.observe(
                _blockTimestamp(),
                secondsAgos,
                slot0.tick,
                slot0.observationIndex,
                liquidity,
                slot0.observationCardinality
        );
    }

    // @inheritdoc IUniswapV3PoolActions
    function increaseObservationCardinalityNext(uint16 observationCardinalityNext)
        external
        override
        lock
        noDelegateCall
    {
        uint16 observationCardinalityNextOld = slot0.observationCardinalityNext; // for the event
        uint16 observationCardinalityNextNew =
            observations.grow(observationCardinalityNextOld, observationCardinalityNext);
        slot0.observationCardinalityNext = observationCardinalityNextNew;
        if (observationCardinalityNextOld != observationCardinalityNextNew)
            emit IncreaseObservationCardinalityNext(observationCardinalityNextOld, observationCardinalityNextNew);
    }

    // @inheritdoc IUniswapV3PoolActions
    /// @dev not locked because it initializes unlocked
    function initialize(uint160 sqrtPriceX96) external override {
        require(slot0.sqrtPriceX96 == 0, 'AI');

        int24 tick = ITickMath(TickMath).getTickAtSqrtRatio(sqrtPriceX96);

        (uint16 cardinality, uint16 cardinalityNext) = observations.initialize(_blockTimestamp());

        slot0 = Slot0({
            sqrtPriceX96: sqrtPriceX96,
            tick: tick,
            observationIndex: 0,
            observationCardinality: cardinality,
            observationCardinalityNext: cardinalityNext,
            feeProtocol: 0,
            unlocked: true
        });

        emit Initialize(sqrtPriceX96, tick);
    }

    // @inheritdoc IUniswapV3PoolActions
    function mint(
        address recipient,
        int24 tickLower,
        int24 tickUpper,
        uint128 amount,
        bytes calldata data
    ) external override lock noDelegateCall returns (uint256 amount0, uint256 amount1) {
        (, bytes memory returnedData) = address(UniswapV3PoolActions).delegatecall(
            abi.encodeWithSignature('mint(address,int24,int24,uint128,bytes)',
                recipient,
                tickLower,
                tickUpper,
                amount,
                data
            )
        );
        (amount0, amount1) = abi.decode(returnedData, (uint256, uint256));
    }

    // @inheritdoc IUniswapV3PoolActions
    function collect(
        address recipient,
        int24 tickLower,
        int24 tickUpper,
        uint128 amount0Requested,
        uint128 amount1Requested
    ) external override lock returns (uint128 amount0, uint128 amount1) {
        (, bytes memory returnedData) = address(UniswapV3PoolActions).delegatecall(
            abi.encodeWithSignature('collect(address,int24,int24,uint128,uint128)',
                recipient,
                tickLower,
                tickUpper,
                amount0Requested,
                amount1Requested
            )
        );
        (amount0, amount1) = abi.decode(returnedData, (uint128, uint128));
    }

    // @inheritdoc IUniswapV3PoolActions
    function burn(
        int24 tickLower,
        int24 tickUpper,
        uint128 amount
    ) external override lock noDelegateCall returns (uint256 amount0, uint256 amount1) {
        (, bytes memory returnedData) = address(UniswapV3PoolActions).delegatecall(
            abi.encodeWithSignature('burn(int24,int24,uint128)',
                tickLower,
                tickUpper,
                amount
            )
        );
        (amount0, amount1) = abi.decode(returnedData, (uint256, uint256));
    }

    // @inheritdoc IUniswapV3PoolActions
    function swap(
        address recipient,
        bool zeroForOne,
        int256 amountSpecified,
        uint160 sqrtPriceLimitX96,
        bytes calldata data
    ) external override noDelegateCall returns (int256 amount0, int256 amount1) {
        (, bytes memory returnedData) = address(UniswapV3PoolActions).delegatecall(
            abi.encodeWithSignature('swap(address,bool,int256,uint160,bytes)',
                recipient,
                zeroForOne,
                amountSpecified,
                sqrtPriceLimitX96,
                data
            )
        );
        (amount0, amount1) = abi.decode(returnedData, (int256, int256));
    }

    // @inheritdoc IUniswapV3PoolActions
    function flash(
        address recipient,
        uint256 amount0,
        uint256 amount1,
        bytes calldata data
    ) external override lock noDelegateCall {
        (,) = address(UniswapV3PoolActions).delegatecall(
            abi.encodeWithSignature('flash(address,uint256,uint256,bytes)',
                recipient,
                amount0,
                amount1,
                data
            )
        );
    }

    // @inheritdoc IUniswapV3PoolOwnerActions
    function setFeeProtocol(uint8 feeProtocol0, uint8 feeProtocol1) external override lock onlyFactoryOwner {
        require(
            (feeProtocol0 == 0 || (feeProtocol0 >= 4 && feeProtocol0 <= 10)) &&
                (feeProtocol1 == 0 || (feeProtocol1 >= 4 && feeProtocol1 <= 10))
        );
        uint8 feeProtocolOld = slot0.feeProtocol;
        slot0.feeProtocol = feeProtocol0 + (feeProtocol1 << 4);
        emit SetFeeProtocol(feeProtocolOld % 16, feeProtocolOld >> 4, feeProtocol0, feeProtocol1);
    }

    // @inheritdoc IUniswapV3PoolOwnerActions
    function collectProtocol(
        address recipient,
        uint128 amount0Requested,
        uint128 amount1Requested
    ) external override lock onlyFactoryOwner returns (uint128 amount0, uint128 amount1) {
        amount0 = amount0Requested > protocolFees.token0 ? uint128(protocolFees.token0) : amount0Requested;
        amount1 = amount1Requested > protocolFees.token1 ? uint128(protocolFees.token1) : amount1Requested;

        if (amount0 > 0) {
            if (amount0 == protocolFees.token0) amount0--; // ensure that the slot is not cleared, for gas savings
            protocolFees.token0 -= amount0;
            TransferHelper.safeTransfer(token0, recipient, amount0);
        }
        if (amount1 > 0) {
            if (amount1 == protocolFees.token1) amount1--; // ensure that the slot is not cleared, for gas savings
            protocolFees.token1 -= amount1;
            TransferHelper.safeTransfer(token1, recipient, amount1);
        }

        emit CollectProtocol(msg.sender, recipient, amount0, amount1);
    }
}
