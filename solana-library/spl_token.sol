import 'solana';

library SplToken {
	address constant tokenProgramId = address"TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
	enum TokenInstruction {
		InitializeMint, // 0
		InitializeAccount, // 1
		InitializeMultisig, // 2
		Transfer, // 3
		Approve, // 4
		Revoke, // 5
		SetAuthority, // 6
		MintTo, // 7
		Burn, // 8
		CloseAccount, // 9
		FreezeAccount, // 10
		ThawAccount, // 11
		TransferChecked, // 12
		ApproveChecked, // 13
		MintToChecked, // 14
		BurnChecked, // 15
		InitializeAccount2, // 16
		SyncNative, // 17
		InitializeAccount3, // 18
		InitializeMultisig2, // 19
		InitializeMint2, // 20
		GetAccountDataSize, // 21
		InitializeImmutableOwner, // 22
		AmountToUiAmount, // 23
		UiAmountToAmount, // 24
		InitializeMintCloseAuthority, // 25
		TransferFeeExtension, // 26
		ConfidentialTransferExtension, // 27
		DefaultAccountStateExtension, // 28
		Reallocate, // 29
		MemoTransferExtension, // 30
		CreateNativeMint // 31
	}

	/// Mint new tokens. The transaction should be signed by the mint authority keypair
	///
	/// @param mint the account of the mint
	/// @param account the token account where the minted tokens should go
	/// @param authority the public key of the mint authority
	/// @param amount the amount of tokens to mint
	function mint_to(address mint, address account, address authority, uint64 amount) internal {
		bytes instr = new bytes(9);

		instr[0] = uint8(TokenInstruction.MintTo);
		instr.writeUint64LE(amount, 1);

		AccountMeta[3] metas = [
			AccountMeta({pubkey: mint, is_writable: true, is_signer: false}),
			AccountMeta({pubkey: account, is_writable: true, is_signer: false}),
			AccountMeta({pubkey: authority, is_writable: true, is_signer: true})
		];

		tokenProgramId.call{accounts: metas}(instr);
	}

	/// Transfer @amount token from @from to @to. The transaction should be signed by the owner
	/// keypair of the from account.
	///
	/// @param from the account to transfer tokens from
	/// @param to the account to transfer tokens to
	/// @param owner the publickey of the from account owner keypair
	/// @param amount the amount to transfer
	function transfer(address from, address to, address owner, uint64 amount) internal {
		bytes instr = new bytes(9);

		instr[0] = uint8(TokenInstruction.Transfer);
		instr.writeUint64LE(amount, 1);

		AccountMeta[3] metas = [
			AccountMeta({pubkey: from, is_writable: true, is_signer: false}),
			AccountMeta({pubkey: to, is_writable: true, is_signer: false}),
			AccountMeta({pubkey: owner, is_writable: true, is_signer: true})
		];

		tokenProgramId.call{accounts: metas}(instr);
	}

	/// Burn @amount tokens in account. This transaction should be signed by the owner.
	///
	/// @param account the acount for which tokens should be burned
	/// @param mint the mint for this token
	/// @param owner the publickey of the account owner keypair
	/// @param amount the amount to transfer
	function burn(address account, address mint, address owner, uint64 amount) internal {
		bytes instr = new bytes(9);

		instr[0] = uint8(TokenInstruction.Burn);
		instr.writeUint64LE(amount, 1);

		AccountMeta[3] metas = [
			AccountMeta({pubkey: account, is_writable: true, is_signer: false}),
			AccountMeta({pubkey: mint, is_writable: true, is_signer: false}),
			AccountMeta({pubkey: owner, is_writable: true, is_signer: true})
		];

		tokenProgramId.call{accounts: metas}(instr);
	}

	/// Approve an amount to a delegate. This transaction should be signed by the owner
	///
	/// @param account the account for which a delegate should be approved
	/// @param delegate the delegate publickey
	/// @param owner the publickey of the account owner keypair
	/// @param amount the amount to approve
	function approve(address account, address delegate, address owner, uint64 amount) internal {
		bytes instr = new bytes(9);

		instr[0] = uint8(TokenInstruction.Approve);
		instr.writeUint64LE(amount, 1);

		AccountMeta[3] metas = [
			AccountMeta({pubkey: account, is_writable: true, is_signer: false}),
			AccountMeta({pubkey: delegate, is_writable: false, is_signer: false}),
			AccountMeta({pubkey: owner, is_writable: false, is_signer: true})
		];

		tokenProgramId.call{accounts: metas}(instr);
	}

	/// Revoke a previously approved delegate. This transaction should be signed by the owner. After
	/// this transaction, no delgate is approved for any amount.
	///
	/// @param account the account for which a delegate should be approved
	/// @param owner the publickey of the account owner keypair
	function revoke(address account, address owner) internal {
		bytes instr = new bytes(1);

		instr[0] = uint8(TokenInstruction.Revoke);

		AccountMeta[2] metas = [
			AccountMeta({pubkey: account, is_writable: true, is_signer: false}),
			AccountMeta({pubkey: owner, is_writable: false, is_signer: true})
		];

		tokenProgramId.call{accounts: metas}(instr);
	}

	/// Get the total supply for the mint, i.e. the total amount in circulation
	/// @param mint the mint for this token
	function total_supply(address mint) internal view returns (uint64) {
		AccountInfo account = get_account_info(mint);

		return account.data.readUint64LE(36);
	}

	/// Get the balance for an account.
	///
	/// @param account the account for which we want to know a balance
	function get_balance(address account) internal view returns (uint64) {
		AccountInfo ai = get_account_info(account);

		return ai.data.readUint64LE(64);
	}

	/// Get the account info for an account. This walks the transaction account infos
	/// and find the account info, or the transaction fails.
	///
	/// @param account the account for which we want to have the acount info.
	function get_account_info(address account) internal view returns (AccountInfo) {
		for (uint64 i = 0; i < tx.accounts.length; i++) {
			AccountInfo ai = tx.accounts[i];
			if (ai.key == account) {
				return ai;
			}
		}

		revert("account missing");
	}
}
