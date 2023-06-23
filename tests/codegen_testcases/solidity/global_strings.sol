// RUN: --target solana --emit llvm-ir
// READ: ServiceRegistrySolana.ll


// BEGIN-CHECK: "math overflow,\0A"
// NOT-CHECK: "math overflow,\0A"

import "solana";
contract ServiceRegistrySolana {
    struct Service {
        address serviceOwner;
        uint64 securityDeposit;
        address multisig;
        bytes32 configHash;
        uint32 threshold;
        uint32 maxNumAgentInstances;
        uint32 numAgentInstances;
        uint8 state;
        uint32[] agentIds;
        uint32[] slots;
        uint64[] bonds;
        address[] operators;
        address[] agentInstances;
        uint32[] agentIdForAgentInstances;
    }

    address public owner;
    address public escrow;
    string public baseURI;
    uint32 public totalSupply;
    string public constant CID_PREFIX = "f01701220";
    uint64 public slashedFunds;
    address public drainer;
    string public constant VERSION = "1.0.0";
    mapping(bytes32 => uint64) public mapOperatorAndServiceIdOperatorBalances;
    mapping (address => address) public mapAgentInstanceOperators;
    mapping (address => bool) public mapMultisigs;
    Service public services;



    constructor(address a, string memory _baseURI)
    {
        
    }

    function changeOwner(address newOwner) external {
        
    }

    function changeDrainer(address newDrainer) external {
        
    }

    function transfer(uint32 serviceId, address newServiceOwner) public {

    }

    function create(
        address serviceOwner,
        bytes32 configHash,
        uint32[] memory agentIds,
        uint32[] memory slots,
        uint64[] memory bonds,
        uint32 threshold
    ) external returns (uint32 serviceId)
    {
        
    }

    function update(
        bytes32 configHash,
        uint32[] memory agentIds,
        uint32[] memory slots,
        uint64[] memory bonds,
        uint32 threshold,
        uint32 serviceId
    ) external returns (bool success)
    {
    }

    function activateRegistration(uint32 serviceId) external payable returns (bool success)
    {

    }

    function registerAgents(
        address operator,
        uint32 serviceId,
        address[] memory agentInstances,
        uint32[] agentIds
    ) external payable returns (bool success)
    {
        
    }

    function deploy(
        uint32 serviceId,
        address multisigImplementation,
        bytes memory data
    ) external returns (address multisig)
    {
        
    }

    function slash(address[] memory agentInstances, uint64[] memory amounts, uint32 serviceId) external
        returns (bool success)
    {
        
    }

    function terminate(uint32 serviceId) external returns (bool success, uint64 refund)
    {
    }

    function unbond(address operator, uint32 serviceId) external returns (bool success, uint64 refund) {

    }

    function getService(uint32 serviceId) external view returns (Service memory service) {
        
    }

    function getAgentParams(uint32 serviceId) external view
        returns (uint32 numAgentIds, uint32[] memory slots, uint64[] memory bonds)
    {
        
    }

    function getAgentInstances(uint32 serviceId) external view
        returns (uint32 numAgentInstances, address[] memory agentInstances)
    {

    }

    function getOperatorBalance(address operator, uint32 serviceId) external view returns (uint64 balance)
    {
    }

    function changeMultisigPermission(address multisig, bool permission) external returns (bool success) {
       
    }

    function drain() external returns (uint64 amount) {
       
    }

    function ownerOf(uint32 id) public view returns (address addr) {
        
    }

    function exists() public view {}
}
