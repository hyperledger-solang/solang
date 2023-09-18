import "solana";

contract B {
    starter ss;
    function declare_contract(address addr) external returns (bool) {
        AccountMeta[1] meta = [
            AccountMeta({pubkey: addr, is_signer: true, is_writable: true})
        ];
        starter g = new starter{accounts: meta}();
        return g.get{accounts: meta}();
    }

    function receive_contract(starter g) public {
        g.flip();
    }

    function return_contract() external returns (starter) {
        starter c = new starter();
        return c;
    }
}



@program_id("CU8sqXecq7pxweQnJq6CARonEApGN2DXcv9ukRBRgnRf")
contract starter {
    bool private value = true;

    modifier test_modifier() {
        print("modifier");
        _;
    }

    constructor() {
        print("Hello, World!");
    }

    function flip() public test_modifier {
            value = !value;
    }

    function get() public view returns (bool) {
            return value;
    }
}

// ---- Expect: diagnostics ----
// error: 4:5-12: contracts are not allowed as types on Solana
// error: 9:9-16: contracts are not allowed as types on Solana
// error: 13:31-38: contracts are not allowed as types on Solana
// error: 17:50-57: contracts are not allowed as types on Solana
