import 'solana';

function standalone(address dataAccount) returns (address) {
    AccountMeta[1] meta = [
        AccountMeta(dataAccount, false, false)
    ];
    return hatchling.root{accounts: meta}();
}

@program_id("5afzkvPkrshqu4onwBCsJccb1swrt4JdAjnpzK8N4BzZ")
contract hatchling {
    string name;
    address private origin;

    constructor(string id, address parent) {
        require(id != "", "name must be provided");
        name = id;
        origin = parent;
    }

    function root() public returns (address) {
        return origin;
    }
}

// ---- Expect: diagnostics ----
// warning: 12:5-16: storage variable 'name' has been assigned, but never read
// warning: 21:5-45: function can be declared 'view'
