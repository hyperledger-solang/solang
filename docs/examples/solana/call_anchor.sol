import "./bobcat.sol";
import "solana";

contract example {
    function test(address a, address b) public {
        // The list of accounts to pass into the Anchor program must be passed
        // as an array of AccountMeta with the correct writable/signer flags set
        AccountMeta[2] am = [
            AccountMeta({pubkey: a, is_writable: true, is_signer: false}),
            AccountMeta({pubkey: b, is_writable: false, is_signer: false})
        ];

        // Any return values are decoded automatically
        int64 res = bobcat.pounce{accounts: am}();
    }
}
