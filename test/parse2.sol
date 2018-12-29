pragma solidity ^0.4.23;

contract foobar {
	struct foo {
		int96 x;
	};
	struct bar {
		bytes32 y;
	};
	event UpdateActiveAgreementCollectionMap(string name, bytes32 key1, address key2) anonymous;
      event LogAgreementCreation(
                bytes32 indexed eventId,
                address agreement_address,
                address archetype_address,
                string name,
                address creator,
                bool is_private,
                uint8   legal_state,
                uint32 max_event_count,
                address formation_process_instance,
                address execution_process_instance,
                bytes32 hoard_address,
                bytes32 hoard_secret,
                bytes32 event_log_hoard_address,
                bytes32 event_log_hoard_secret
        );

enum ParameterType {BOOLEAN, STRING, NUMBER, DATE, DATETIME, MONETARY_AMOUNT, USER_ORGANIZATION, CONTRACT_ADDRESS, SIGNING_PARTY};
	function getId() external view returns (bytes32) {
		break;
		continue;
		102 * 4;
	}
}
