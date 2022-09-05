// SPDX-License-Identifier: Apache-2.0

// DISCLAIMER: This file is an example of how to mint and transfer NFTs on Solana. It is not production ready and has not been audited for security.
// Use it at your own risk.

import '../../solana-library/spl_token.sol';

contract SimpleCollectible {
    // tokenCounter creates a unique ID for each new minted NFT
    uint32 private tokenCounter;
    // The authority responsible for managing mints.
    address private mintAuthority;

    struct NFTOwner {
        bool exists;
        address owner;
        address ownerTokenAccount;
        address mintAccount;
    }

    struct NFTData {
        NFTOwner ownerData;
        string uri;
    }

    event NFTMinted(address owner, address mintAccount, uint32 nftId);
    event NFTSold(address from, address to, uint32 nftId);

    mapping(uint32 => NFTData) private nftInfo;

    constructor (address _mintAuthority) {
        tokenCounter = 0;
        // For every mint on Solana, a mint authority is needed to sign it
        mintAuthority = _mintAuthority;
    }

    /// Create a new NFT and associated it to a URI
    ///
    /// @param tokenURI a URI that leads to the NFT resource
    /// @param mintAccount an account that saves the total supply of a token
    /// @param owner the account that is going to own the newly minted NFT
    /// @param ownerTokenAccount the associated token account for the @param owner and the @param mintAccount
    ///
    /// return: an unique identifier to the minted NFT
    function createCollectible(string memory tokenURI, address mintAccount, address owner, address ownerTokenAccount) public returns (uint32) {
        uint32 new_item_id = tokenCounter;
        tokenCounter++;
        // An NFT on Solana is a SPL-Token with only one minted token.
        // The mintAccount saves information about the token (e.g. quantity of tokens avaialble)
        SplToken.mint_to(mintAccount, ownerTokenAccount, mintAuthority, 1);

        NFTOwner ownerData = NFTOwner(true, owner, ownerTokenAccount, mintAccount);
        nftInfo[new_item_id] = NFTData(ownerData, tokenURI);

        // Save on blockchain records information about the created token
        emit NFTMinted(owner, mintAccount, new_item_id);
        return new_item_id;
    }

    /// Transfer ownership of an NFT from one account to another
    ///
    /// @param tokenId the NFT's unique identifer generated when it was minted
    /// @param newOwner the account for the new NFT owner
    /// @param newOwnerTokenAccount the associated token account for the @param newOwner
    function transferOwnership(uint32 tokenId, address newOwner, address newOwnerTokenAccount) public {
        NFTData storage data = nftInfo[tokenId];
        if(!data.ownerData.exists) {
            return;
        }
        
        // To transfer the ownership of a token, we need the current owner and the new owner. The payer account is the account used to derive
        // the correspondent token account in TypeScript.
        SplToken.transfer(data.ownerData.ownerTokenAccount, newOwnerTokenAccount, data.ownerData.owner, 1);
        data.ownerData.owner = newOwner;
        data.ownerData.ownerTokenAccount = newOwnerTokenAccount;

        emit NFTSold(data.ownerData.ownerTokenAccount, newOwnerTokenAccount, tokenId);
    }

    // Returns the URI of an NFT
    //
    // @param nftId the unique identifier generated when the NFT was minted
    function getNftUri(uint32 nftId) public view returns (string memory) {
        if (nftInfo[nftId].ownerData.exists) {
            return nftInfo[nftId].uri;
        }

        return "";
    }


    // Returns the owner, the token account and the mint account of an NFT
    //
    // @param nftID the unique identified generated when the NFT was minted
    function getNftOwner(uint32 nftId) public view returns (NFTOwner) {
        return nftInfo[nftId].ownerData;
    }
}
