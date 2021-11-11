// SPDX-License-Identifier: MIT

pragma solidity 0.6.12;

import "../sushiswap/libraries/SafeERC20.sol";
import "./interfaces/Allocatable.sol";
import "./interfaces/IBokky.sol";
import "./interfaces/IFauceteer.sol";
import "./interfaces/IPoly.sol";

contract Faucet {
  using SafeERC20 for IERC20;

  address[] bokky;

  address[] compound;

  IFauceteer fauceteer;

  IERC20 sushi;

  function initFaucet(
    address[] memory _bokky,
    address[] memory _compound,
    IFauceteer _fauceteer,
    IERC20 _sushi
  ) public {
    bokky = _bokky;
    compound = _compound;
    fauceteer = _fauceteer;
    sushi = _sushi;
  }

  function _dump(address token) private {
    uint256 balance = IERC20(token).balanceOf(address(this));
    IERC20(token).safeTransfer(msg.sender, balance);
  }

  function drip() public {
    uint256 id = block.chainid;

    sushi.safeTransfer(msg.sender, sushi.balanceOf(address(this)) / 10000); // 0.01%

    for (uint256 i = 0; i < bokky.length; i++) {
      address b = bokky[i];
      IBokky(b).drip();
      _dump(b);
    }

    if (id == 3) {
      IPoly(0x96A62428509002a7aE5F6AD29E4750d852A3f3D7).getTokens(5000 * 1e18);
      _dump(0x96A62428509002a7aE5F6AD29E4750d852A3f3D7);
    }

    if (id == 3 || id == 42) {
      for (uint256 j = 0; j < compound.length; j++) {
        address c = compound[j];
        fauceteer.drip(c);
        _dump(c);
      }
    }

    if (id == 4 || id == 5) {
      for (uint256 k = 0; k < compound.length; k++) {
        address c = compound[k];
        Allocatable(c).allocateTo(
          msg.sender,
          1000 * (10**uint256(IERC20(c).safeDecimals()))
        );
      }
    }
  }
}
