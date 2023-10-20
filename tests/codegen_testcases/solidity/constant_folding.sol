// RUN: --target polkadot --emit cfg

contract CodeWithJD {
    mapping(address => uint256) balances;

    // BEGIN-CHECK: CodeWithJD::CodeWithJD::function::totalSupply
    function totalSupply() public view returns (uint256) {
        // CHECK: load storage slot(hex"b55fba97e5495840b2400ab391e4362b96f1173f44a58442cdd3f776b62832ad") ty:uint256
        return balances[address(0x00)];
    }
}

contract Enum {
    enum Lender {
        USDT,
        USDC,
        DAI
    }
    mapping(Lender => address) lenders;

    // BEGIN-CHECK: Enum::Enum::constructor::7d622c65
    constructor(address usdt, address usdc, address dai) {
        lenders[Lender.USDT] = usdt;
        lenders[Lender.USDC] = usdc;
        lenders[Lender.DAI] = dai;

        // TODO / FIXME:
        // We need an unused variable detection pass in codegen, and run all optimization until the CFG converges.
        // This will get rid of the unused temp variable assignments below.

        // CHECK: ty:address %temp.17 = (arg #0)
        // CHECK: store storage slot(hex"f31349e4056d5e5c8ce6d8359404f2ca89b2a6884691bff0f55ce7629f869af3") ty:address = (arg #0)
        // CHECK: ty:address %temp.18 = (arg #1)
        // CHECK: store storage slot(hex"e062efc721ea447b5e3918617d57f26130f3d8bc01b883eed1efcb4864d73ac1") ty:address = (arg #1)
        // CHECK: ty:address %temp.19 = (arg #2)
        // CHECK: store storage slot(hex"b2573af2738ebd4810a3198e92bab190f29b8718f1d5ed1b83e468f2bb322d10") ty:address = (arg #2)
    }

    function foo(Lender lender) public view returns (address) {
        return lenders[lender];
    }
}
