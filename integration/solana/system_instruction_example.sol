import '../../solana-library/system_instruction.sol';

contract TestingInstruction {
    function create_account(address from, address to, uint64 lamports, uint64 space, address owner) public {
        SystemInstruction.create_account(from, to, lamports, space, owner);
    }

    function create_account_with_seed(address from, address to, address base, string seed, uint64 lamports, uint64 space, address owner) public {
        SystemInstruction.create_account_with_seed(from, to, base, seed, lamports, space, owner);
    }

    function assign(address account, address owner) public {
        SystemInstruction.assign(account, owner);
    }

    function assign_with_seed(address account, address base, string seed, address owner) public {
        SystemInstruction.assign_with_seed(account, base, seed, owner);
    }

    function transfer(address from, address to, uint64 lamports) public {
        SystemInstruction.transfer(from, to, lamports);
    }

    function transfer_with_seed(address from_pubkey, address from_base, string seed, address from_owner, address to_pubkey, uint64 lamports) public {
        SystemInstruction.transfer_with_seed(from_pubkey, from_base, seed, from_owner, to_pubkey, lamports);
    }

    function allocate(address pub_key, uint64 space) public {
        SystemInstruction.allocate(pub_key, space);
    }

    function allocate_with_seed(address addr, address base, string seed, uint64 space, address owner) public {
        SystemInstruction.allocate_with_seed(addr, base, seed, space, owner);
    }

    function create_nonce_account_with_seed(address from, address nonce, address base, string seed, address authority, uint64 lamports) public {
        SystemInstruction.create_nonce_account_with_seed(from, nonce, base, seed, authority, lamports);
    }

    function create_nonce_account(address from, address nonce, address authority, uint64 lamports) public {
        SystemInstruction.create_nonce_account(from, nonce, authority, lamports);
    }

    function advance_nonce_account(address nonce, address authorized) public {
        SystemInstruction.advance_nonce_account(nonce, authorized);
    }

    function withdraw_nonce_account(address nonce, address authority, address to, uint64 lamports) public {
        SystemInstruction.withdraw_nonce_account(nonce, authority, to, lamports);
    }

    function authorize_nonce_account(address nonce, address authority, address new_authority) public {
        SystemInstruction.authorize_nonce_account(nonce, authority, new_authority);
    }

    // This is not available on Solana v1.9.15
    // function upgrade_nonce_account(address nonce) public {
    //     SystemInstruction.upgrade_nonce_account(nonce);
    // }
}