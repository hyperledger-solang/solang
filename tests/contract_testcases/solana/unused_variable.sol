import "solana";

contract MyTest {
    function test_this(uint32 i, address addr) public view returns (uint32) {
        AccountInfo info ; tx.accounts[i];
        if (info.key == addr) {
            return 0;
        } else if (info.lamports == 90) {
            return 1;
        } else if (info.data.length == 5) {
            return info.data.readUint32LE(4);
        } else if (info.owner == addr) {
            return 3;
        } else if (info.rent_epoch == 45) {
            return 4;
        } else if (info.is_signer) {
            return 5;
        } else if (info.is_writable) {
            return 6;
        } else if (info.executable) {
            return 7;
        }
    }
}


// ---- Expect: diagnostics ----
// warning: 4:31-32: function parameter 'i' is unused
// error: 5:21-25: Variable 'info' is undefined
// 	note 6:13-17: Variable read before being defined
// 	note 8:20-24: Variable read before being defined
// 	note 12:20-24: Variable read before being defined
// 	note 11:20-24: Variable read before being defined
// 	note 14:20-24: Variable read before being defined
// 	note 16:20-24: Variable read before being defined
// 	note 18:20-24: Variable read before being defined
// 	note 20:20-24: Variable read before being defined
