import "solana";

contract c {
    function test() public payable returns (address) {
        for (uint32 i = 0; i < tx.accounts.length; i++) {
            AccountInfo ai = tx.accounts[i];

            if (ai.key == address(this)) {
                return ai.owner;
            }
        }

        revert("account not found");
    }
}
