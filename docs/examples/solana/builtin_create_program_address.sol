import {create_program_address} from 'solana';

contract pda {
    address token = address"TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";

    function create_pda(bytes seed2) public returns (address) {
        return create_program_address(["kabang", seed2], token);
    }
}
