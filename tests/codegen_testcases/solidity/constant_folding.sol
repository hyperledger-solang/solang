// RUN: --target substrate --emit cfg

contract CodeWithJD {
    mapping(address => uint256) balances;
    // BEGIN-CHECK: CodeWithJD::CodeWithJD::function::totalSupply
    function totalSupply() public view returns (uint256) {
        // CHECK: load storage slot(hex"b55fba97e5495840b2400ab391e4362b96f1173f44a58442cdd3f776b62832ad") ty:uint256
        return balances[address(0x00)];
    }
}
