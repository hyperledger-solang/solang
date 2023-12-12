pragma solidity ^0.8.10;


/// @author Max Campbell (https://github.com/maxall41), RafaCypherpunk (https://github.com/RafaCypherpunk)
contract Property {
  using Counters for Counters.Counter;
  Counters.Counter private _tokenIds;

  mapping(uint256 => uint256) public pricePerShare_;
  mapping(address => uint256) public valueLocked_;
  mapping(uint256 => address) public tokenDeployers_;
  mapping(uint256 => uint256) public sellingTokens_;
  mapping(uint256 => uint256) public buyingTokens_;

  event MintProperty(uint256 id);

  constructor() ERC1155("https://api.vestana.io/api/product/get/{id}.json") {}

  receive() external payable {
    valueLocked_[msg.sender] = valueLocked_[msg.sender] + msg.value;
  }

  function mintProperty(
    uint256 _shares,
    uint256 _pricePerShare,
    uint256 _sharesForSale
  ) public {
    uint256 newPropertyId = _tokenIds.current();
    _tokenIds.increment();
    _mint(msg.sender, newPropertyId, _shares, "");
    pricePerShare_[newPropertyId] = _pricePerShare;
    tokenDeployers_[newPropertyId] = msg.sender;
    sellingTokens_[newPropertyId] = _sharesForSale;
    emit MintProperty(newPropertyId);
  }

  function getTokenOwner(uint256 _id) public view returns (address payable) {
    return payable(tokenDeployers_[_id]);
  }

  function getPricePerShare(uint256 _id) public view returns (uint256) {
    return pricePerShare_[_id];
  }

  /// @dev Used to purchase shares
  function purchaseShares(uint256 _shares, uint256 _id) public payable {
    /// @dev Get the owner of this token
    address payable owner = getTokenOwner(_id);
    /// @dev Get the price per share of this token
    uint256 _pricePerShare = getPricePerShare(_id);
    /// @dev Mae sure the contract can spend the owner's tokens
    require(
      this.isApprovedForAll(owner, address(this)) == true,
      "Owner has incorrect permissions"
    );
    /// @dev Make sure the purchaser has enough shares
    require(msg.value >= _pricePerShare * _shares, "Not enough");
    /// @dev Make sure there are shares available for purchase
    require(sellingTokens_[_id] >= _shares, "No more shares available");
    /// @dev Charges purchaser for shares
    owner.transfer(_pricePerShare * _shares);
    /// @dev Transfers purchased shares to purchaser
    /// @note This will fail if the owner has no more shares they want to sell
    this.safeTransferFrom(tokenDeployers_[_id], msg.sender, _id, _shares, "");
  }

  function setSellingShares(uint256 _newSharesToSell, uint256 _id) public {
    require(msg.sender == tokenDeployers_[_id], "You are not the owner");
    sellingTokens_[_id] = _newSharesToSell;
  }

  function setBuyingShares(uint256 _newSharesToSell, uint256 _id) public {
    require(msg.sender == tokenDeployers_[_id], "You are not the owner");
    buyingTokens_[_id] = _newSharesToSell;
  }

  function sellShares(uint256 shares_, uint256 _id) public {
    /// @dev Get the price per share
    uint256 _pricePerShare = getPricePerShare(_id);
    ///@dev Get the owner
    address _owner = getTokenOwner(_id);
    /// @dev Make sure the owner wants to sell these shares
    require(buyingTokens_[_id] >= shares_, "No buyback capacity");
    /// @dev Make sure the sender has enough shares
    require(
      this.balanceOf(msg.sender, _id) >= shares_,
      "Not enough shares to sell"
    );
    /// @dev Make sure the owner can afford this
    require(
      valueLocked_[_owner] >= shares_ * _pricePerShare,
      "Seller does not have enough assets"
    );
    /// @dev Charge purchaser shares
    this.safeTransferFrom(msg.sender, _owner, _id, shares_, "");
    /// @dev Send the purchaser the native token
    payable(msg.sender).transfer(shares_ * _pricePerShare);
  }
}

// ---- Expect: diagnostics ----
// error: 6:22-30: 'Counters' not found
// error: 7:3-11: 'Counters' not found
// error: 17:17-24: 'ERC1155' not found
// error: 28:29-38: '_tokenIds' not found
// error: 53:12-28: unknown function 'isApprovedForAll'
// error: 86:12-21: unknown function 'balanceOf'
