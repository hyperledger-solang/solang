pragma solidity ^0.8.0;
import "./UniswapV2Pair.sol";

contract Creator {
    address public pair;

    constructor() public {
        pair = address(new UniswapV2Pair());
    }

}

