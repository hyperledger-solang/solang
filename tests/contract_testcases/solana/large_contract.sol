
// This is a test for issue #1367: https://github.com/hyperledger-solang/solang/issues/1367
// This contract used to generate so many calls to 'vector_new' for allocating error strings that it caused a LLVM ERROR: Branch target out of insn range
import "solana";

/// @title Service Registry Solana - Smart contract for registering services on the Solana chain.
/// @dev Underlying canonical agents and components are not checked for their validity since they are set up on the L1 mainnet.
///      The architecture is optimistic, in the sense that service owners are assumed to reference existing and relevant agents.
/// @author Aleksandr Kuperman - <aleksandr.kuperman@valory.xyz>
/// @author Andrey Lebedev - <andrey.lebedev@valory.xyz>
contract ServiceRegistrySolana {
    event OwnerUpdated(address indexed owner);
    event BaseURIChanged(string baseURI);
    event DrainerUpdated(address indexed drainer);
    event Deposit(address indexed sender, uint64 amount);
    event Refund(address indexed receiver, uint64 amount);
    event CreateService(uint32 indexed serviceId, bytes32 configHash);
    event UpdateService(uint32 indexed serviceId, bytes32 configHash);
    event RegisterInstance(address indexed operator, uint32 indexed serviceId, address indexed agentInstance, uint32 agentId);
    event CreateMultisigWithAgents(uint32 indexed serviceId, address indexed multisig);
    event ActivateRegistration(uint32 indexed serviceId);
    event TerminateService(uint32 indexed serviceId);
    event OperatorSlashed(uint64 amount, address indexed operator, uint32 indexed serviceId);
    event OperatorUnbond(address indexed operator, uint32 indexed serviceId);
    event DeployService(uint32 indexed serviceId);
    event Drain(address indexed drainer, uint64 amount);

    enum ServiceState {
        NonExistent,
        PreRegistration,
        ActiveRegistration,
        FinishedRegistration,
        Deployed,
        TerminatedBonded
    }

    // Service parameters
    struct Service {
        address serviceOwner;
        // Registration activation deposit
        // This is enough for 1b+ ETH or 1e27
        uint64 securityDeposit;
        // Multisig address for agent instances
        address multisig;
        // IPFS hashes pointing to the config metadata
        bytes32 configHash;
        // Agent instance signers threshold: must no less than ceil((n * 2 + 1) / 3) of all the agent instances combined
        // This number will be enough to have ((2^32 - 1) * 3 - 1) / 2, which is bigger than 6.44b
        uint32 threshold;
        // Total number of agent instances. We assume that the number of instances is bounded by 2^32 - 1
        uint32 maxNumAgentInstances;
        // Actual number of agent instances. This number is less or equal to maxNumAgentInstances
        uint32 numAgentInstances;
        // Service state
        ServiceState state;
        // Canonical agent Ids for the service. Individual agent Id is bounded by the max number of agent Id
        uint32[] agentIds;
        uint32[] slots;
        uint64[] bonds;
        address[] operators;
        address[] agentInstances;
        uint32[] agentIdForAgentInstances;
    }

    // The public key for the storage authority (owner) that should sign every change to the contract
    address public owner;
    address public escrow;
    // Base URI
    string public baseURI;
    // Service counter
    uint32 public totalSupply;
    // Reentrancy lock
    uint32 internal _locked = 1;
    // To better understand the CID anatomy, please refer to: https://proto.school/anatomy-of-a-cid/05
    // CID = <multibase_encoding>multibase_encoding(<cid-version><multicodec><multihash-algorithm><multihash-length><multihash-hash>)
    // CID prefix = <multibase_encoding>multibase_encoding(<cid-version><multicodec><multihash-algorithm><multihash-length>)
    // to complement the multibase_encoding(<multihash-hash>)
    // multibase_encoding = base16 = "f"
    // cid-version = version 1 = "0x01"
    // multicodec = dag-pb = "0x70"
    // multihash-algorithm = sha2-256 = "0x12"
    // multihash-length = 256 bits = "0x20"
    string public constant CID_PREFIX = "f01701220";
    // The amount of funds slashed. This is enough for 1b+ ETH or 1e27
    uint64 public slashedFunds;
    // Drainer address: set by the government and is allowed to drain ETH funds accumulated in this contract
    address public drainer;
    // Service registry version number
    string public constant VERSION = "1.0.0";
    // Map of hash(operator address and serviceId) => agent instance bonding / escrow balance
    mapping(bytes32 => uint64) public mapOperatorAndServiceIdOperatorBalances;
    // Map of agent instance address => service id it is registered with and operator address that supplied the instance
    mapping (address => address) public mapAgentInstanceOperators;
    // Map of policy for multisig implementations
    mapping (address => bool) public mapMultisigs;
    // Set of services
    Service[type(uint32).max] public services;


    /// @dev Service registry constructor.
    /// @param _owner Agent contract symbol.
    /// @param _baseURI Agent registry token base URI.
    constructor(address _owner, string memory _baseURI)
    {
        owner = _owner;
        baseURI = _baseURI;
    }

    /// Requires the signature of the metadata authority.
    function requireSigner(address authority) internal view {
        for(uint32 i=0; i < tx.accounts.length; i++) {
            if (tx.accounts[i].key == authority) {
                require(tx.accounts[i].is_signer, "the authority account must sign the transaction");
                return;
            }
        }

        revert("The authority is missing");
    }

    /// @dev Changes the owner address.
    /// @param newOwner Address of a new owner.
    function changeOwner(address newOwner) external {
        // Check for the metadata authority
        requireSigner(owner);

        // Check for the zero address
        if (newOwner == address(0)) {
            revert("Zero Address");
        }

        owner = newOwner;
        emit OwnerUpdated(newOwner);
    }

    /// @dev Changes the drainer.
    /// @param newDrainer Address of a drainer.
    function changeDrainer(address newDrainer) external {
        // Check for the metadata authority
        requireSigner(owner);

        // Check for the zero address
        if (newDrainer == address(0)) {
            revert("ZeroAddress");
        }

        drainer = newDrainer;
        emit DrainerUpdated(newDrainer);
    }

    function transfer(uint32 serviceId, address newServiceOwner) public {
        // Check for the service authority
        address serviceOwner = services[serviceId].serviceOwner;
        requireSigner(serviceOwner);

        services[serviceId].serviceOwner = newServiceOwner;

    }

    /// @dev Going through basic initial service checks.
    /// @param configHash IPFS hash pointing to the config metadata.
    /// @param agentIds Canonical agent Ids.
    /// @param slots Set of agent instances number for each agent Id.
    /// @param bonds Corresponding set of required bonds to register an agent instance in the service.
    function _initialChecks(
        bytes32 configHash,
        uint32[] memory agentIds,
        uint32[] memory slots,
        uint64[] memory bonds
    ) private pure
    {
        // Check for the non-zero hash value
        if (configHash == 0) {
            revert("ZeroValue");
        }

        // Checking for non-empty arrays and correct number of values in them
        if (agentIds.length == 0 || agentIds.length != slots.length || agentIds.length != bonds.length) {
            revert("WrongArrayLength");
        }

        // Check for duplicate canonical agent Ids
        uint32 lastId = 0;
        for (uint32 i = 0; i < agentIds.length; i++) {
            if (agentIds[i] < (lastId + 1)) {
                revert("WrongAgentId");
            }
            lastId = agentIds[i];
        }

        // Check that there are no zero number of slots for a specific canonical agent id and no zero registration bond
        for (uint32 i = 0; i < agentIds.length; i++) {
            if (slots[i] == 0 || bonds[i] == 0) {
                revert("ZeroValue");
            }
        }
    }

    /// @dev Sets the service data.
    /// @param service A service instance to fill the data for.
    /// @param agentIds Canonical agent Ids.
    /// @param slots Set of agent instances number for each agent Id.
    /// @param bonds Corresponding set of required bonds to register an agent instance in the service.
    function _setServiceData(
        Service memory service,
        uint32[] memory agentIds,
        uint32[] memory slots,
        uint64[] memory bonds
    ) private
    {
        // Security deposit
        uint64 securityDeposit = 0;
        // Add canonical agent Ids for the service and the slots map
        service.agentIds = agentIds;
        service.slots = slots;
        service.bonds = bonds;
        for (uint32 i = 0; i < agentIds.length; i++) {
            service.maxNumAgentInstances += slots[i];
            // Security deposit is the maximum of the canonical agent registration bond
            if (bonds[i] > securityDeposit) {
                securityDeposit = bonds[i];
            }
        }
        service.securityDeposit = securityDeposit;

        // Check for the correct threshold: no less than ceil((n * 2 + 1) / 3) of all the agent instances combined
        uint32 checkThreshold = service.maxNumAgentInstances * 2 + 1;
        if (checkThreshold % 3 == 0) {
            checkThreshold = checkThreshold / 3;
        } else {
            checkThreshold = checkThreshold / 3 + 1;
        }
        if (service.threshold < checkThreshold || service.threshold > service.maxNumAgentInstances) {
            revert("WrongThreshold");
        }
    }

    /// @dev Creates a new service.
    /// @notice If agentIds are not sorted in ascending order then the function that executes initial checks gets reverted.
    /// @param serviceOwner Individual that creates and controls a service.
    /// @param configHash IPFS hash pointing to the config metadata.
    /// @param agentIds Canonical agent Ids in a sorted ascending order.
    /// @param slots Set of agent instances number for each agent Id.
    /// @param bonds Corresponding set of required bonds to register an agent instance in the service.
    /// @param threshold Signers threshold for a multisig composed by agent instances.
    /// @return serviceId Created service Id.
    function create(
        address serviceOwner,
        bytes32 configHash,
        uint32[] memory agentIds,
        uint32[] memory slots,
        uint64[] memory bonds,
        uint32 threshold
    ) external returns (uint32 serviceId)
    {
        // Reentrancy guard
        if (_locked > 1) {
            revert("ReentrancyGuard");
        }
        _locked = 2;

        // Check for the non-empty service owner address
        if (serviceOwner == address(0)) {
            revert("ZeroAddress");
        }

        // Execute initial checks
        _initialChecks(configHash, agentIds, slots, bonds);

        // Create a new service Id
        serviceId = totalSupply;
        serviceId++;

        // Set high-level data components of the service instance
        Service memory service;
        // Updating high-level data components of the service
        service.threshold = threshold;
        // Assigning the initial hash
        service.configHash = configHash;
        // Set the initial service state
        service.state = ServiceState.PreRegistration;

        // Set service data
        _setServiceData(service, agentIds, slots, bonds);

        // Mint the service instance to the service owner and record the service structure
        service.serviceOwner = serviceOwner;

        services[serviceId] = service;
        totalSupply = serviceId;

        emit CreateService(serviceId, configHash);

        _locked = 1;
    }

    /// @dev Updates a service in a CRUD way.
    /// @param configHash IPFS hash pointing to the config metadata.
    /// @param agentIds Canonical agent Ids in a sorted ascending order.
    /// @notice If agentIds are not sorted in ascending order then the function that executes initial checks gets reverted.
    /// @param slots Set of agent instances number for each agent Id.
    /// @param bonds Corresponding set of required bonds to register an agent instance in the service.
    /// @param threshold Signers threshold for a multisig composed by agent instances.
    /// @param serviceId Service Id to be updated.
    /// @return success True, if function executed successfully.
    function update(
        bytes32 configHash,
        uint32[] memory agentIds,
        uint32[] memory slots,
        uint64[] memory bonds,
        uint32 threshold,
        uint32 serviceId
    ) external returns (bool success)
    {
        Service memory service = services[serviceId];

        // Check for the service authority
        address serviceOwner = service.serviceOwner;
        requireSigner(serviceOwner);

        if (service.state != ServiceState.PreRegistration) {
            revert("WrongServiceState");
        }

        // Execute initial checks
        _initialChecks(configHash, agentIds, slots, bonds);

        // Updating high-level data components of the service
        service.threshold = threshold;
        service.maxNumAgentInstances = 0;

        // Check if the previous hash is the same / hash was not updated
        bytes32 lastConfigHash = service.configHash;
        if (lastConfigHash != configHash) {
            service.configHash = configHash;
        }

        // Set service data and record the modified service struct
        _setServiceData(service, agentIds, slots, bonds);
        services[serviceId] = service;

        emit UpdateService(serviceId, configHash);
        success = true;
    }

    /// @dev Activates the service.
    /// @param serviceId Correspondent service Id.
    /// @return success True, if function executed successfully.
    function activateRegistration(uint32 serviceId) external payable returns (bool success)
    {
        Service storage service = services[serviceId];

        // Check for the service authority
        address serviceOwner = service.serviceOwner;
        requireSigner(serviceOwner);

        // Service must be inactive
        if (service.state != ServiceState.PreRegistration) {
            revert("ServiceMustBeInactive");
        }

        // TODO: Need to check that the balance of the escrow for the serviceOwner has enough balance
//        if (msg.value != service.securityDeposit) {
//            revert IncorrectRegistrationDepositValue(msg.value, service.securityDeposit, serviceId);
//        }

        // Activate the agent instance registration
        service.state = ServiceState.ActiveRegistration;

        emit ActivateRegistration(serviceId);
        success = true;
    }

    /// @dev Registers agent instances.
    /// @param operator Address of the operator.
    /// @param serviceId Service Id to register agent instances for.
    /// @param agentInstances Agent instance addresses.
    /// @param agentIds Canonical Ids of the agent correspondent to the agent instance.
    /// @return success True, if function executed successfully.
    function registerAgents(
        address operator,
        uint32 serviceId,
        address[] memory agentInstances,
        uint32[] memory agentIds
    ) external payable returns (bool success)
    {
        // Check if the length of canonical agent instance addresses array and ids array have the same length
        if (agentInstances.length != agentIds.length) {
            revert("WrongArrayLength");
        }

        Service storage service = services[serviceId];
        // The service has to be active to register agents
        if (service.state != ServiceState.ActiveRegistration) {
            revert("WrongServiceState");
        }

        // Check for the sufficient amount of bond fee is provided
        uint32 numAgents = agentInstances.length;
        uint64 totalBond = 0;
        uint32[] memory serviceAgentIds = service.agentIds;
        for (uint32 i = 0; i < numAgents; ++i) {
            // Check if canonical agent Id exists in the service
            uint32 idx = 0;
            for (; idx < serviceAgentIds.length; ++idx) {
                if (serviceAgentIds[idx] == agentIds[i]) {
                    break;
                }
            }
            // If the index reached the length of service agent Ids, the agent Id does not exist in the service
            if (idx == service.agentIds.length) {
                revert("AgentNotInService");
            }
            totalBond += service.bonds[idx];
        }
        // TODO: Check the escrow balance
//        if (msg.value != totalBond) {
//            revert("IncorrectAgentBondingValue");
//        }

        // Operator address must not be used as an agent instance anywhere else
        if (mapAgentInstanceOperators[operator] != address(0)) {
            revert("WrongOperator");
        }

        for (uint32 i = 0; i < numAgents; ++i) {
            address agentInstance = agentInstances[i];
            uint32 agentId = agentIds[i];

            // Operator address must be different from agent instance one
            if (operator == agentInstance) {
                revert("WrongOperator");
            }

            // Check if the agent instance is already engaged with another service
            if (mapAgentInstanceOperators[agentInstance] != address(0)) {
                revert("AgentInstanceRegistered");
            }

            // Check if there is an empty slot for the agent instance in this specific service
            uint32 numRegAgentInstances = 0;
            for (uint32 j = 0; j < service.agentIdForAgentInstances.length; ++j) {
                if (service.agentIdForAgentInstances[j] == agentId) {
                    numRegAgentInstances++;
                }
            }
            if (numRegAgentInstances == service.slots[i]) {
                revert("AgentInstancesSlotsFilled");
            }

            // Copy new operator and agent instance information to the new one
            address[] memory oldOperators = service.operators;
            address[] memory oldAgentInstances = service.agentInstances;
            uint32[] memory oldAgentIdForAgentInstances = service.agentIdForAgentInstances;
            address[] memory newOperators = new address[](oldOperators.length + 1);
            address[] memory newAgentInstances = new address[](oldOperators.length + 1);
            uint32[] memory newAgentIdForAgentInstances = new uint32[](oldOperators.length + 1);
            for (uint32 j = 0; j < oldOperators.length; ++j) {
                newOperators[j] = oldOperators[j];
                newAgentInstances[j] = oldAgentInstances[j];
                newAgentIdForAgentInstances[j] = oldAgentIdForAgentInstances[j];
            }

            // Add agent instance and operator and set the instance engagement
            newOperators[oldOperators.length] = operator;
            newAgentInstances[oldOperators.length] = agentInstance;
            newAgentIdForAgentInstances[oldOperators.length] = agentId;
            // Update service arrays
            service.operators = newOperators;
            service.agentInstances = newAgentInstances;
            service.agentIdForAgentInstances = newAgentIdForAgentInstances;
            // Increase the total number of agent instances in the service
            service.numAgentInstances++;
            mapAgentInstanceOperators[agentInstance] = operator;

            emit RegisterInstance(operator, serviceId, agentInstance, agentId);
        }

        // If the service agent instance capacity is reached, the service becomes finished-registration
        if (service.numAgentInstances == service.maxNumAgentInstances) {
            service.state = ServiceState.FinishedRegistration;
        }

        // Update operator's bonding balance
        bytes32 operatorServiceIdHash = keccak256(abi.encode(operator, serviceId));
        mapOperatorAndServiceIdOperatorBalances[operatorServiceIdHash] += totalBond;

        emit Deposit(operator, totalBond);
        success = true;
    }

    /// @dev Creates multisig instance controlled by the set of service agent instances and deploys the service.
    /// @param serviceId Correspondent service Id.
    /// @param multisigImplementation Multisig implementation address.
    /// @param data Data payload for the multisig creation.
    /// @return multisig Address of the created multisig.
    function deploy(
        uint32 serviceId,
        address multisigImplementation,
        bytes memory data
    ) external returns (address multisig)
    {
        // Reentrancy guard
        if (_locked > 1) {
            revert("ReentrancyGuard");
        }
        _locked = 2;

        Service storage service = services[serviceId];

        // Check for the service authority
        address serviceOwner = service.serviceOwner;
        requireSigner(serviceOwner);

        // Check for the whitelisted multisig implementation
        if (!mapMultisigs[multisigImplementation]) {
            revert("UnauthorizedMultisig");
        }

        if (service.state != ServiceState.FinishedRegistration) {
            revert("WrongServiceState");
        }

        // Get all agent instances for the multisig
        address[] memory agentInstances = service.agentInstances;

        // TODO: Understand the multisig workflow
        // Create a multisig with agent instances
        multisig = address(0);//IMultisig(multisigImplementation).create(agentInstances, service.threshold, data);

        service.multisig = multisig;
        service.state = ServiceState.Deployed;

        emit CreateMultisigWithAgents(serviceId, multisig);
        emit DeployService(serviceId);

        _locked = 1;
    }

    /// @dev Slashes a specified agent instance.
    /// @param agentInstances Agent instances to slash.
    /// @param amounts Correspondent amounts to slash.
    /// @param serviceId Service Id.
    /// @return success True, if function executed successfully.
    function slash(address[] memory agentInstances, uint64[] memory amounts, uint32 serviceId) external
        returns (bool success)
    {
        // Check if the service is deployed
        // Since we do not kill (burn) services, we want this check to happen in a right service state.
        // If the service is deployed, it definitely exists and is running. We do not want this function to be abused
        // when the service was deployed, then terminated, then in a sleep mode or before next deployment somebody
        // could use this function and try to slash operators.
        Service storage service = services[serviceId];
        if (service.state != ServiceState.Deployed) {
            revert("WrongServiceState");
        }

        // Check for the array size
        if (agentInstances.length != amounts.length) {
            revert("WrongArrayLength");
        }

        // Only the multisig of a correspondent address can slash its agent instances
        requireSigner(service.multisig);

        // Loop over each agent instance
        uint32 numInstancesToSlash = agentInstances.length;
        for (uint32 i = 0; i < numInstancesToSlash; ++i) {
            // Get the service Id from the agentInstance map
            address operator = mapAgentInstanceOperators[agentInstances[i]];
            // Slash the balance of the operator, make sure it does not go below zero
            bytes32 operatorServiceIdHash = keccak256(abi.encode(operator, serviceId));
            uint64 balance = mapOperatorAndServiceIdOperatorBalances[operatorServiceIdHash];
            if ((amounts[i] + 1) > balance) {
                // We cannot add to the slashed amount more than the balance of the operator
                slashedFunds += balance;
                balance = 0;
            } else {
                slashedFunds += amounts[i];
                balance -= amounts[i];
            }
            mapOperatorAndServiceIdOperatorBalances[operatorServiceIdHash] = balance;

            emit OperatorSlashed(amounts[i], operator, serviceId);
        }
        success = true;
    }

    /// @dev Terminates the service.
    /// @param serviceId Service Id to be updated.
    /// @return success True, if function executed successfully.
    /// @return refund Refund to return to the service owner.
    function terminate(uint32 serviceId) external returns (bool success, uint64 refund)
    {
        // Reentrancy guard
        if (_locked > 1) {
            revert("ReentrancyGuard");
        }
        _locked = 2;

        Service storage service = services[serviceId];

        // Check for the service authority
        address serviceOwner = service.serviceOwner;
        requireSigner(serviceOwner);

        // Check if the service is already terminated
        if (service.state == ServiceState.PreRegistration || service.state == ServiceState.TerminatedBonded) {
            revert("WrongServiceState");
        }
        // Define the state of the service depending on the number of bonded agent instances
        if (service.numAgentInstances > 0) {
            service.state = ServiceState.TerminatedBonded;
        } else {
            service.state = ServiceState.PreRegistration;
        }

        // Return registration deposit back to the service owner
        refund = service.securityDeposit;
        // TODO: Figure out the escrow release
//        // By design, the refund is always a non-zero value, so no check is needed here fo that
//        (bool result, ) = serviceOwner.call{value: refund}("");
//        if (!result) {
//            revert TransferFailed(address(0), address(this), serviceOwner, refund);
//        }

        emit Refund(serviceOwner, refund);
        emit TerminateService(serviceId);
        success = true;

        _locked = 1;
    }

    /// @dev Unbonds agent instances of the operator from the service.
    /// @param operator Operator of agent instances.
    /// @param serviceId Service Id.
    /// @return success True, if function executed successfully.
    /// @return refund The amount of refund returned to the operator.
    function unbond(address operator, uint32 serviceId) external returns (bool success, uint64 refund) {
        // Reentrancy guard
        if (_locked > 1) {
            revert("ReentrancyGuard");
        }
        _locked = 2;

        // Checks if the operator address is not zero
        if (operator == address(0)) {
            revert("ZeroAddress");
        }

        Service storage service = services[serviceId];
        // Service can only be in the terminated-bonded state or expired-registration in order to proceed
        if (service.state != ServiceState.TerminatedBonded) {
            revert("WrongServiceState");
        }

        // Check for the operator and unbond all its agent instances
        uint32 numAgentsUnbond = 0;
        for (uint32 j = 0; j < service.operators.length; ++j) {
            if (operator == service.operators[j]) {
                // Get the agent Id of the agent instance
                uint32 agentId = service.agentIdForAgentInstances[j];
                // Remove the operator from the set of service operators
                service.operators[j] = address(0);
                numAgentsUnbond++;

                // Add the bond associated with the agent Id to the refund
                for (uint32 idx = 0; idx < service.agentIds.length; ++idx) {
                    if (service.agentIds[idx] == agentId) {
                        refund += uint64(service.bonds[idx]);
                        break;
                    }
                }
            }
        }
        if (numAgentsUnbond == 0) {
            revert("OperatorHasNoInstances");
        }

        // Subtract number of unbonded agent instances
        service.numAgentInstances -= numAgentsUnbond;
        // When number of instances is equal to zero, all the operators have unbonded and the service is moved into
        // the PreRegistration state, from where it can be updated / start registration / get deployed again
        if (service.numAgentInstances == 0) {
            delete service.operators;
            delete service.agentInstances;
            delete service.agentIdForAgentInstances;
            service.state = ServiceState.PreRegistration;
        }
        // else condition is redundant here, since the service is either in the TerminatedBonded state, or moved
        // into the PreRegistration state and unbonding is not possible before the new TerminatedBonded state is reached

        // Calculate the refund
        bytes32 operatorServiceIdHash = keccak256(abi.encode(operator, serviceId));
        uint64 balance = mapOperatorAndServiceIdOperatorBalances[operatorServiceIdHash];
        // This situation is possible if the operator was slashed for the agent instance misbehavior
        if (refund > balance) {
            refund = balance;
        }

        // Refund the operator
        if (refund > 0) {
            // Operator's balance is essentially zero after the refund
            mapOperatorAndServiceIdOperatorBalances[operatorServiceIdHash] = 0;
            // TODO: Figure out the escrow release
//            // Send the refund
//            (bool result, ) = operator.call{value: refund}("");
//            if (!result) {
//                revert("TransferFailed");
//            }
            emit Refund(operator, refund);
        }

        emit OperatorUnbond(operator, serviceId);
        success = true;

        _locked = 1;
    }

    /// @dev Gets the service instance.
    /// @param serviceId Service Id.
    /// @return service Corresponding Service struct.
    function getService(uint32 serviceId) external view returns (Service memory service) {
        service = services[serviceId];
    }

    /// @dev Gets service agent parameters: number of agent instances (slots) and a bond amount.
    /// @param serviceId Service Id.
    /// @return numAgentIds Number of canonical agent Ids in the service.
    /// @return slots Set of agent instances number for each agent Id.
    /// @return bonds Corresponding set of required bonds to register an agent instance in the service.
    function getAgentParams(uint32 serviceId) external view
        returns (uint32 numAgentIds, uint32[] memory slots, uint64[] memory bonds)
    {
        Service memory service = services[serviceId];
        numAgentIds = service.agentIds.length;
        slots = new uint32[](numAgentIds);
        bonds = new uint64[](numAgentIds);
        for (uint32 i = 0; i < numAgentIds; ++i) {
            slots[i] = service.slots[i];
            bonds[i] = service.bonds[i];
        }
    }

//    /// @dev Lists all the instances of a given canonical agent Id if the service.
//    /// @param serviceId Service Id.
//    /// @param agentId Canonical agent Id.
//    /// @return numAgentInstances Number of agent instances.
//    /// @return agentInstances Set of agent instances for a specified canonical agent Id.
//    function getInstancesForAgentId(uint32 serviceId, uint32 agentId) external view
//        returns (uint32 numAgentInstances, address[] memory agentInstances)
//    {
//        numAgentInstances = mapServiceAndAgentIdAgentInstances[serviceId][agentId].length;
//        agentInstances = new address[](numAgentInstances);
//        for (uint32 i = 0; i < numAgentInstances; i++) {
//            agentInstances[i] = mapServiceAndAgentIdAgentInstances[serviceId][agentId][i];
//        }
//    }

    /// @dev Gets service agent instances.
    /// @param serviceId ServiceId.
    /// @return numAgentInstances Number of agent instances.
    /// @return agentInstances Pre-allocated list of agent instance addresses.
    function getAgentInstances(uint32 serviceId) external view
        returns (uint32 numAgentInstances, address[] memory agentInstances)
    {
        Service memory service = services[serviceId];
        agentInstances = service.agentInstances;
        numAgentInstances = agentInstances.length;
    }

    /// @dev Gets the operator's balance in a specific service.
    /// @param operator Operator address.
    /// @param serviceId Service Id.
    /// @return balance The balance of the operator.
    function getOperatorBalance(address operator, uint32 serviceId) external view returns (uint64 balance)
    {
        bytes32 operatorServiceIdHash = keccak256(abi.encode(operator, serviceId));
        balance = mapOperatorAndServiceIdOperatorBalances[operatorServiceIdHash];
    }

    /// @dev Controls multisig implementation address permission.
    /// @param multisig Address of a multisig implementation.
    /// @param permission Grant or revoke permission.
    /// @return success True, if function executed successfully.
    function changeMultisigPermission(address multisig, bool permission) external returns (bool success) {
        // Check for the contract authority
        requireSigner(owner);

        if (multisig == address(0)) {
            revert("ZeroAddress");
        }
        mapMultisigs[multisig] = permission;
        success = true;
    }

    /// @dev Drains slashed funds.
    /// @return amount Drained amount.
    function drain() external returns (uint64 amount) {
        // Reentrancy guard
        if (_locked > 1) {
            revert("ReentrancyGuard");
        }
        _locked = 2;

        // Check for the drainer address
        requireSigner(drainer);

        // Drain the slashed funds
        amount = slashedFunds;
        if (amount > 0) {
            slashedFunds = 0;
            // TODO: Figure out the amount send
            // Send the amount
//            (bool result, ) = msg.sender.call{value: amount}("");
//            if (!result) {
//                revert TransferFailed(address(0), address(this), msg.sender, amount);
//            }
            emit Drain(drainer, amount);
        }

        _locked = 1;
    }

    function ownerOf(uint32 serviceId) public view returns (address) {
        return services[serviceId].serviceOwner;
    }
    
    /// @dev Checks for the service existence.
    /// @notice Service counter starts from 1.
    /// @param serviceId Service Id.
    /// @return true if the service exists, false otherwise.
    function exists(uint32 serviceId) public view returns (bool) {
        return serviceId > 0 && serviceId < (totalSupply + 1);
    }

    /// @dev Sets service base URI.
    /// @param bURI Base URI string.
    function setBaseURI(string memory bURI) external {
        requireSigner(owner);

        // Check for the zero value
        if (bytes(bURI).length == 0) {
            revert("Zero Value");
        }

        baseURI = bURI;
        emit BaseURIChanged(bURI);
    }

    /// @dev Gets the valid service Id from the provided index.
    /// @notice Service counter starts from 1.
    /// @param id Service counter.
    /// @return serviceId Service Id.
    function tokenByIndex(uint32 id) external view returns (uint32 serviceId) {
        serviceId = id + 1;
        if (serviceId > totalSupply) {
            revert("Overflow");
        }
    }

    // Open sourced from: https://stackoverflow.com/questions/67893318/solidity-how-to-represent-bytes32-as-string
    /// @dev Converts bytes16 input data to hex16.
    /// @notice This method converts bytes into the same bytes-character hex16 representation.
    /// @param data bytes16 input data.
    /// @return result hex16 conversion from the input bytes16 data.
    function _toHex16(bytes16 data) internal pure returns (bytes32 result) {
        result = bytes32 (data) & 0xFFFFFFFFFFFFFFFF000000000000000000000000000000000000000000000000 |
        (bytes32 (data) & 0x0000000000000000FFFFFFFFFFFFFFFF00000000000000000000000000000000) >> 64;
        result = result & 0xFFFFFFFF000000000000000000000000FFFFFFFF000000000000000000000000 |
        (result & 0x00000000FFFFFFFF000000000000000000000000FFFFFFFF0000000000000000) >> 32;
        result = result & 0xFFFF000000000000FFFF000000000000FFFF000000000000FFFF000000000000 |
        (result & 0x0000FFFF000000000000FFFF000000000000FFFF000000000000FFFF00000000) >> 16;
        result = result & 0xFF000000FF000000FF000000FF000000FF000000FF000000FF000000FF000000 |
        (result & 0x00FF000000FF000000FF000000FF000000FF000000FF000000FF000000FF0000) >> 8;
        result = (result & 0xF000F000F000F000F000F000F000F000F000F000F000F000F000F000F000F000) >> 4 |
        (result & 0x0F000F000F000F000F000F000F000F000F000F000F000F000F000F000F000F00) >> 8;
        result = bytes32 (0x3030303030303030303030303030303030303030303030303030303030303030 +
        uint256 (result) +
            (uint256 (result) + 0x0606060606060606060606060606060606060606060606060606060606060606 >> 4 &
            0x0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F) * 39);
    }

    /// @dev Returns service token URI.
    /// @notice Expected multicodec: dag-pb; hashing function: sha2-256, with base16 encoding and leading CID_PREFIX removed.
    /// @param serviceId Service Id.
    /// @return Service token URI string.
    function tokenURI(uint32 serviceId) public view returns (string memory) {
        bytes32 serviceHash = services[serviceId].configHash;
        // Parse 2 parts of bytes32 into left and right hex16 representation, and concatenate into string
        // adding the base URI and a cid prefix for the full base16 multibase prefix IPFS hash representation
        return string(abi.encodePacked(baseURI, CID_PREFIX, _toHex16(bytes16(serviceHash)),
            _toHex16(bytes16(serviceHash << 128))));
    }
}

// ---- Expect: diagnostics ----
// warning: 204:5-209:14: function can be declared 'pure'
// warning: 500:22-26: function parameter 'data' is unused
// warning: 525:26-40: local variable 'agentInstances' is unused
// warning: 867:31-35: function parameter 'data' is unused