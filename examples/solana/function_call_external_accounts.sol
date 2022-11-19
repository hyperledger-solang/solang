import {AccountMeta} from 'solana';

contract SplToken {
    address constant tokenProgramId = address"TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
    address constant SYSVAR_RENT_PUBKEY = address"SysvarRent111111111111111111111111111111111";

    struct InitializeMintInstruction {
        uint8 instruction;
        uint8 decimals;
        address mintAuthority;
        uint8 freezeAuthorityOption;
        address freezeAuthority;
    }

    function create_mint_with_freezeauthority(uint8 decimals, address mintAuthority, address freezeAuthority) public {
        InitializeMintInstruction instr = InitializeMintInstruction({
            instruction: 0,
            decimals: decimals,
            mintAuthority: mintAuthority,
            freezeAuthorityOption: 1,
            freezeAuthority: freezeAuthority
        });

        AccountMeta[2] metas = [
            AccountMeta({pubkey: instr.mintAuthority, is_writable: true, is_signer: false}),
            AccountMeta({pubkey: SYSVAR_RENT_PUBKEY, is_writable: false, is_signer: false})
        ];

        tokenProgramId.call{accounts: metas}(instr);
    }
}
