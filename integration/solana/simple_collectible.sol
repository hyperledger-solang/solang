// SPDX-License-Identifier: Apache-2.0

// DISCLAIMER: This file is an example of how to mint and transfer NFTs on Solana. It is not production ready and has not been audited for security.
// Use it at your own risk.

import '../../solana-library/spl_token.sol';

contract SimpleCollectible {
    // On Solana, the mintAccount represents the type of token created. It saves how many tokens exist in circulation.
    address private mintAccount;
    // The public key for the authority that should sign every change to the NFT's URI
    address private metadataAuthority;
    // A resource identifier to access the NFT. It could be any other data to be saved on the blockchain
    string private uri;

    // These events log on the blockchain transactions made with this NFT
    event NFTMinted(address owner, address mintAccount);
    event NFTSold(address from, address to);

    // The mint account will identify the NFT in this example
    constructor (address _mintAccount, address _metadataAuthority) {
        mintAccount = _mintAccount;
        metadataAuthority = _metadataAuthority;
    }

    /// Create a new NFT
    @mutableAccount(mintAccount) // The account of the mint. Its address must be the same as that of the 'mintAccount' contract variable.
    @mutableAccount(ownerTokenAccount) // The owner's associated token account
    @signer(mintAuthority) // The account that signs each new mint
    function createCollectible() external {
        SplToken.TokenAccountData token_data = SplToken.get_token_account_data(tx.accounts.ownerTokenAccount);

        SplToken.MintAccountData mint_data = SplToken.get_mint_account_data(tx.accounts.mintAccount);
        // Ensure the supply is zero. Otherwise, this is not an NFT.
        assert(mint_data.supply == 0);

        // An NFT on Solana is a SPL-Token with only one minted token.
        // The token account saves the owner of the tokens minted with the mint account, the respective mint account and the number
        // of tokens the owner account owns
        SplToken.mint_to(tx.accounts.mintAccount.key, tx.accounts.ownerTokenAccount.key, tx.accounts.mintAuthority.key, 1);

        // Set the mint authority to null. This prevents that any other new tokens be minted, ensuring we have an NFT.
        SplToken.remove_mint_authority(tx.accounts.mintAccount.key, tx.accounts.mintAuthority.key);

        // Log on blockchain records information about the created token
        emit NFTMinted(token_data.owner, token_data.mintAccount);
    }

    /// Transfer ownership of this NFT from one account to another
    /// This function only wraps the innate SPL transfer, which can be used outside this contract.
    /// However, the difference here is the event 'NFTSold' exclusive to this function
    @mutableAccount(oldTokenAccount) // The token account for the current owner
    @mutableAccount(newTokenAccount) // The token account for the new owner
    @signer(oldOwner)
    function transferOwnership() external {
        // The current owner does not need to be the caller of this functions, but they need to sign the transaction
        // with their private key.
        SplToken.TokenAccountData old_data = SplToken.get_token_account_data(tx.accounts.oldTokenAccount);
        SplToken.TokenAccountData new_data = SplToken.get_token_account_data(tx.accounts.newTokenAccount);

        // To transfer the ownership of a token, we need the current owner and the new owner. The payer account is the account used to derive
        // the correspondent token account in TypeScript.
        SplToken.transfer(
            tx.accounts.oldTokenAccount.key, 
            tx.accounts.newTokenAccount.key, 
            tx.accounts.oldOwner.key, 1);
        emit NFTSold(old_data.owner, new_data.owner);
    }

    /// Return the URI of this NFT
    function getNftUri() public view returns (string memory) {
        return uri;
    }

    /// Check if an NFT is owned by @param owner
    ///
    /// @param owner the account whose ownership we want to verify
    @account(tokenAccount) // The owner's associated token account
    function isOwner(address owner) external view returns (bool) {
        SplToken.TokenAccountData data = SplToken.get_token_account_data(tx.accounts.tokenAccount);

        return owner == data.owner && mintAccount == data.mintAccount && data.balance == 1;
    }

    /// Updates the NFT URI
    /// The metadata authority must sign the transaction so that the update can succeed.
    ///
    /// @param newUri a new URI for the NFT
    @signer(metadataSigner) // The metadata authority that can authorize changes in the NFT data.
    function updateNftUri(string newUri) external {
        require(tx.accounts.metadataSigner.is_signer, "the metadata authority must sign the transaction");
        assert(tx.accounts.metadataSigner.key == metadataAuthority);
        uri = newUri;
    }
}
