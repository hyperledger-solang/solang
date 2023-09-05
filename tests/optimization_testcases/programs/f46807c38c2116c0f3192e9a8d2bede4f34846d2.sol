import "solana";

contract c {
    function test(uint32 index) public payable returns (uint8) {
        for (uint32 i = 0; i < tx.accounts.length; i++) {
            AccountInfo ai = tx.accounts[i];

            if (ai.key == address(this)) {
                return ai.data[index];
            }
        }

        revert("account not found");
    }

    function test2() public payable returns (uint32, uint32) {
        for (uint32 i = 0; i < tx.accounts.length; i++) {
            AccountInfo ai = tx.accounts[i];

            if (ai.key == tx.accounts.dataAccount.key) {
                return (ai.data.readUint32LE(1), ai.data.length);
            }
        }

        revert("account not found");
    }
}
