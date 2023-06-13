import "./Tornado.sol";

contract NativeTornado is Tornado {
    constructor(
        IVerifier _verifier,
        IHasher _hasher,
        uint128 _denomination,
        uint32 _merkleTreeHeight
    ) Tornado(_verifier, _hasher, _denomination, _merkleTreeHeight) {}

    function _processDeposit() internal view override {
        require(
            msg.value == denomination,
            "Send exactly {} along with transaction".format(denomination)
        );
    }

    function _processWithdraw(
        address payable _recipient,
        address payable _relayer,
        uint128 _fee,
        uint128 _refund
    ) internal override {
        require(msg.value == 0, "Message value is supposed to be zero");
        require(_refund == 0, "Refund value is supposed to be zero");
        require(_fee == 0, "Relayers are not used with this PoC");
        require(uint256(_relayer) == 0, "Relayers are not used with this PoC");
        _recipient.transfer(denomination);
    }
}
