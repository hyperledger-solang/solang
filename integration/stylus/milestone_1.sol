// SPDX-License-Identifier: UNLICENSED
pragma solidity >=0.8.0;

// `read_args` is called during function dispatch. `write_result` is called when an
// external/public function returns.
contract C {
    uint public state = 0;

    function set(uint value) external {
        // storage_cache_bytes32
        // storage_flush_cache
        state = value;
    }

    function get() external view returns (uint) {
        return state;
    }

    function msg_sender() external view returns (address) {
        // msg_sender
        return msg.sender;
    }

    function tx_origin() external view returns (address) {
        // tx_origin
        return tx.origin;
    }

    function test() public {
        // call_contract
        (bool call_success, ) = address(this).call(
            abi.encodeWithSignature("set(uint256)", 2)
        );
        assert(call_success);

        // delegate_call_contract
        (bool delegatecall_success, ) = address(this).delegatecall(
            abi.encodeWithSignature("set(uint256)", 3)
        );
        assert(delegatecall_success);

        // static_call_contract
        // read_return_data
        // return_data_size
        uint result = this.get();
        assert(result == 3);

        // msg_sender
        assert(this.msg_sender() == address(this));

        // tx_origin
        address origin = this.tx_origin();
        print("origin = {}".format(origin));
        assert(origin != address(this));
    }
}
