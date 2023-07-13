
        import 'solana';

        contract pda {
            function create_pda(bool cond) public returns (address) {
                address program_id = address"BPFLoaderUpgradeab1e11111111111111111111111";
                address addr = create_program_address(["Talking", "Cats"], program_id);
                if (cond) {
                    return create_program_address(["Talking", "Squirrels"], program_id);
                } else {
                    return addr;
                }
            }

            function create_pda2(bytes a, bytes b) public returns (address) {
                address program_id = address"BPFLoaderUpgradeab1e11111111111111111111111";

                return create_program_address([a, b], program_id);
            }

            function create_pda2_bump(bool cond) public returns (address, bytes1) {
                address program_id = address"BPFLoaderUpgradeab1e11111111111111111111111";
                (address addr, bytes1 bump) = try_find_program_address(["bar", hex"01234567"], program_id);

                if (cond) {
                    return try_find_program_address(["foo", hex"01234567"], program_id);
                } else {
                    return (addr, bump);
                }
            }
        }