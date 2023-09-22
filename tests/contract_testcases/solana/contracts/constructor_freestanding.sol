import 'solana';

function standalone(address dataAccount) returns (address) {
    AccountMeta[1] meta = [
        AccountMeta(dataAccount, false, false)
    ];

    hatchling.new{accounts: meta}("my_id", dataAccount);
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
// error: 8:5-56: constructors not allowed in free standing functions
