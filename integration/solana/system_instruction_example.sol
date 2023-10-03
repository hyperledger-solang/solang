// SPDX-License-Identifier: Apache-2.0

import '../../solana-library/system_instruction.sol';

contract TestingInstruction {

    @mutableSigner(from)
    @mutableSigner(to)
    function create_account(uint64 lamports, uint64 space, address owner) external {
        SystemInstruction.create_account(tx.accounts.from.key, tx.accounts.to.key, lamports, space, owner);
    }

    @mutableSigner(from)
    @mutableAccount(to)
    @signer(base)
    function create_account_with_seed(string seed, uint64 lamports, uint64 space, address owner) external {
        SystemInstruction.create_account_with_seed(
            tx.accounts.from.key, tx.accounts.to.key, tx.accounts.base.key, seed, lamports, space, owner);
    }

    @mutableSigner(assignAccount)
    function assign(address owner) external {
        SystemInstruction.assign(tx.accounts.assignAccount.key, owner);
    }

    @mutableAccount(assignAccount)
    @signer(base)
    function assign_with_seed(string seed, address owner) external {
        SystemInstruction.assign_with_seed(tx.accounts.assignAccount.key, tx.accounts.base.key, seed, owner);
    }

    @mutableSigner(from)
    @mutableAccount(to)
    function transfer(uint64 lamports) external {
        SystemInstruction.transfer(tx.accounts.from.key, tx.accounts.to.key, lamports);
    }

    @mutableAccount(fromKey)
    @signer(fromBase)
    @mutableAccount(toKey)
    function transfer_with_seed(string seed, address from_owner, uint64 lamports) external {
        SystemInstruction.transfer_with_seed(
            tx.accounts.fromKey.key, 
            tx.accounts.fromBase.key, 
            seed, 
            from_owner, 
            tx.accounts.toKey.key, 
            lamports);
    }

    @mutableSigner(accKey)
    function allocate(uint64 space) external {
        SystemInstruction.allocate(tx.accounts.accKey.key, space);
    }

    @mutableAccount(accKey)
    @signer(base)
    function allocate_with_seed(string seed, uint64 space, address owner) external {
        SystemInstruction.allocate_with_seed(
            tx.accounts.accKey.key, 
            tx.accounts.base.key, 
            seed, 
            space, 
            owner);
    }

    @mutableSigner(from)
    @mutableAccount(nonce)
    @signer(base)
    function create_nonce_account_with_seed(string seed, address authority, uint64 lamports) external {
        SystemInstruction.create_nonce_account_with_seed(
            tx.accounts.from.key, 
            tx.accounts.nonce.key,
            tx.accounts.base.key,
            seed, 
            authority,
            lamports);
    }

    @mutableSigner(from)
    @mutableSigner(nonce)
    function create_nonce_account(address authority, uint64 lamports) external {
        SystemInstruction.create_nonce_account(tx.accounts.from.key,
         tx.accounts.nonce.key, authority, lamports);
    }

    @mutableAccount(nonce)
    function advance_nonce_account(address authorized) external {
        SystemInstruction.advance_nonce_account(
            tx.accounts.nonce.key, authorized);
    }

    @mutableAccount(nonce)
    @mutableAccount(to)
    @signer(authority)
    function withdraw_nonce_account(uint64 lamports) external {
        SystemInstruction.withdraw_nonce_account(
            tx.accounts.nonce.key, 
            tx.accounts.authority.key, 
            tx.accounts.to.key, lamports);
    }

    @mutableAccount(nonce)
    @signer(authority)
    function authorize_nonce_account(address new_authority) external {
        SystemInstruction.authorize_nonce_account(
            tx.accounts.nonce.key, 
            tx.accounts.authority.key, new_authority);
    }

    // This is not available on Solana v1.9.15
    // function upgrade_nonce_account(address nonce) external {
    //     SystemInstruction.upgrade_nonce_account(nonce);
    // }
}