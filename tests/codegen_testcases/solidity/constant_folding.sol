// RUN: --target ewasm --emit cfg

contract CodeWithJD {
    mapping(address => uint256) balances;
    // BEGIN-CHECK: CodeWithJD::CodeWithJD::function::totalSupply
    function totalSupply() public view returns (uint256) {
        // CHECK: load storage slot(hex"57644ed870b18b050a0801bb92210bbd4c1bff6200ff20e5e51ab4aae9546da8") ty:uint256
        return balances[address(0x00)];
    }
}
