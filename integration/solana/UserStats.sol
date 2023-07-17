// SPDX-License-Identifier: Apache-2.0

@program_id("GLXybr8w3egyd8HpnHJEy6vQUoXyD3uGzjoUcAnmjQwx")
contract UserStats {
    string private name;
    uint16 private level;
    uint8 private bump;

    // The constructor initializes the PDA hash table for a user.
    @payer(wallet)
    @seed("user-stats")
    @space(250)
    constructor(@seed bytes user_key, @bump uint8 _bump, string _name, uint16 _level) {
        name = _name;
        level = _level;
        bump = _bump;
    }

    // Change the name saved in the data account
    function change_user_name(string new_name) external {
        name = new_name;
    }

    // Change the level saved in the data account
    function change_level(uint16 new_level) external {
        level = new_level;
    }

    // Read the information from the data account
    function return_stats() external view returns (string memory, uint16, uint8) {
        return (name, level, bump);
    }
}