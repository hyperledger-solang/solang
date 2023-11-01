// RUN: --target solana --emit cfg

contract DTron {
    // BEGIN-CHECK: DTron::DTron::function::moneyDeposit__address:_uint256:
    function moneyDeposit(address[] memory thanksCash, uint256[] memory amount) public payable {
        // CHECK: ty:address payable %receiver = address payable(address((sext uint256 (trunc uint160 uint256((load (subscript address[] (arg #0)[uint32 2])))))))
        address payable receiver = payable(address(uint160(thanksCash[2])));
        assert(receiver == address(this));
    }
}