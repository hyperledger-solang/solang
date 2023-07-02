import "polkadot";

// Partial mock of the ink! "mother" integration test.
contract IsContractOracle {
    function contract_oracle(address _address) public view returns (bool) {
        return is_contract(_address);
    }
}
