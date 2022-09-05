// SPDX-License-Identifier: Apache-2.0

// DISCLAIMER: This file is an example of how to mint and transfer NFTs on Solana. It is not production ready and has not been audited for security.
// Use it at your own risk.

import '../../solana-library/spl_token.sol';
import '../../solana-library/system_instruction.sol';

contract SimpleCollectible {
    // Contract variables
    uint32 private tokenCounter;
    address private mint_authority;
    address private payer;

    struct NFTData {
        address owner;
        address mint_account;
        string uri;
        bool exists;
    }

    event NFTMinted(address owner, address mint_account, uint32 nft_id);
    event NFTSold(address from, address to, uint32 nft_id);

    mapping(uint32 => NFTData) private nft_information;

    constructor (address _mint_authority) {
        tokenCounter = 0;
        // For every mint on Solana, a mint authority is needed to sign each mint
        mint_authority = _mint_authority;
    }

    // Sets the payer account that is going to pay for every transference
    function set_payer(address _payer) public {
        payer = _payer;
    }

    function createCollectible(string memory tokenURI, address mint_account, address owner) public returns (uint32) {
        uint32 newItemId = tokenCounter;
        tokenCounter++;
        // An NFT on Solana is a SPL-Token with only one minted token.
        // The mint_account saves information about the token (e.g. quantity of tokens avaialble)
        SplToken.mint_to(mint_account, owner, mint_authority, 1);

        nft_information[newItemId] = NFTData(owner, mint_account, tokenURI, true);

        // Save on blockchain records information about the created token
        emit NFTMinted(owner, mint_account, newItemId);
        return newItemId;
    }

    function transfer_ownership(uint32 tokenId, address new_owner) public {
        NFTData storage data = nft_information[tokenId];
        if(!data.exists) {
            return;
        }
        
        // To transfer the ownership of a token, we need the current owner and the new owner. The payer account is the account used to derive
        // the correspondent token account in JavaScript.
        SplToken.transfer(data.owner, new_owner, payer, 1);
        emit NFTSold(data.owner, new_owner, tokenId);
        data.owner = new_owner;
    }

    // Returns the URI of an NFT
    function get_nft_uri(uint32 nftId) public view returns (string memory) {
        if (nft_information[nftId].exists) {
            return nft_information[nftId].uri;
        }

        return "";
    }

    // Returns the owner and the mint account of an NFT
    function get_nft_owner(uint32 nft_id) public view returns (address, address) {
        return (nft_information[nft_id].owner, nft_information[nft_id].mint_account);
    }
}