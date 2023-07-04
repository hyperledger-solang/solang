// RUN: --target polkadot --emit cfg
contract Ownable {
    uint256 public _ext;

    // BEGIN-CHECK: Ownable::Ownable::function::_msgData
    function _msgData() internal view returns (bytes memory) {
        // CHECK: return (builtin Calldata ())
        return msg.data;
    }
}
