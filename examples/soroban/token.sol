/// SPDX-License-Identifier: Apache-2.0

contract token {
    address public admin;
    uint32 public decimals;
    string public name;
    string public symbol;

    constructor(
        address _admin,
        string memory _name,
        string memory _symbol,
        uint32 _decimals
    ) {
        admin = _admin;
        name = _name;
        symbol = _symbol;
        decimals = _decimals;
    }

    mapping(address => int128) public balances;
    mapping(address => mapping(address => int128)) public allowances;

    function mint(address to, int128 amount) public {
        require(amount >= 0, "Amount must be non-negative");
        admin.requireAuth();
        setBalance(to, balance(to) + amount);
    }

    function approve(address owner, address spender, int128 amount) public {
        require(amount >= 0, "Amount must be non-negative");
        owner.requireAuth();
        allowances[owner][spender] = amount;
    }

    function transfer(address from, address to, int128 amount) public {
        require(amount >= 0, "Amount must be non-negative");
        from.requireAuth();
        require(balance(from) >= amount, "Insufficient balance");
        setBalance(from, balance(from) - amount);
        setBalance(to, balance(to) + amount);
    }

    function transfer_from(
        address spender,
        address from,
        address to,
        int128 amount
    ) public {
        require(amount >= 0, "Amount must be non-negative");
        spender.requireAuth();
        require(balance(from) >= amount, "Insufficient balance");
        require(allowance(from, spender) >= amount, "Insufficient allowance");
        setBalance(from, balance(from) - amount);
        setBalance(to, balance(to) + amount);
        allowances[from][spender] -= amount;
    }

    function burn(address from, int128 amount) public {
        require(amount >= 0, "Amount must be non-negative");
        require(balance(from) >= amount, "Insufficient balance");
        from.requireAuth();
        setBalance(from, balance(from) - amount);
    }

    function burn_from(address spender, address from, int128 amount) public {
        require(amount >= 0, "Amount must be non-negative");
        spender.requireAuth();
        require(balance(from) >= amount, "Insufficient balance");
        require(allowance(from, spender) >= amount, "Insufficient allowance");
        setBalance(from, balance(from) - amount);
        allowances[from][spender] -= amount;
    }

    function setBalance(address addr, int128 amount) internal {
        balances[addr] = amount;
    }

    function balance(address addr) public view returns (int128) {
        return balances[addr];
    }

    function allowance(
        address owner,
        address spender
    ) public view returns (int128) {
        return allowances[owner][spender];
    }
}
