contract c {
    function test(address addr) public view returns (uint64) {
        return addr.balance;
    }
}

contract c1 {
    function send(address payable addr, uint64 amount) public returns (bool) {
        return addr.send(amount);
    }
}

 contract c2 {
    function transfer(address payable addr, uint64 amount) public {
        addr.transfer(amount);
    }
}

// ---- Expect: diagnostics ----
// error: 3:16-20: balance is not available on Solana. Use tx.accounts.account_name.lamports to fetch the balance.
// error: 9:16-33: method 'send' not available on Solana. Use the lamports field from the AccountInfo struct directly to operate on balances.
// error: 15:9-30: method 'transfer' not available on Solana. Use the lamports field from the AccountInfo struct directly to operate on balances.
