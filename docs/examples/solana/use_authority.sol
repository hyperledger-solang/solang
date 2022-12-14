import 'solana';

contract AuthorityExample {
    address authority;
    uint64 counter;

    modifier needs_authority() {
        for (uint64 i = 0; i < tx.accounts.length; i++) {
            AccountInfo ai = tx.accounts[i];

            if (ai.key == authority && ai.is_signer) {
                _;
                return;
            }
        }

        print("not signed by authority");
        revert();
    }

    constructor(address initial_authority) {
        authority = initial_authority;
    }

    function set_new_authority(address new_authority) needs_authority public {
        authority = new_authority;
    }

    function inc() needs_authority public {
        counter += 1;
    }

    function get() public view returns (uint64) {
        return counter;
    }
}