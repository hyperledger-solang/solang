// https://tornado.cash
/*
 * d888888P                                           dP              a88888b.                   dP
 *    88                                              88             d8'   `88                   88
 *    88    .d8888b. 88d888b. 88d888b. .d8888b. .d888b88 .d8888b.    88        .d8888b. .d8888b. 88d888b.
 *    88    88'  `88 88'  `88 88'  `88 88'  `88 88'  `88 88'  `88    88        88'  `88 Y8ooooo. 88'  `88
 *    88    88.  .88 88       88    88 88.  .88 88.  .88 88.  .88 dP Y8.   .88 88.  .88       88 88    88
 *    dP    `88888P' dP       dP    dP `88888P8 `88888P8 `88888P' 88  Y88888P' `88888P8 `88888P' dP    dP
 * ooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooo
 */

// SPDX-License-Identifier: MIT
pragma solidity ^0.7.0;

import "./MerkleTreeWithHistory.sol";

// Reentrancy is disabled by default anyways
//import "@openzeppelin/contracts/security/ReentrancyGuard.sol";

interface IVerifier {
    function verifyProof(bytes memory _proof, uint256[6] memory _input) external returns (bool);
}

abstract contract Tornado is MerkleTreeWithHistory {
    IVerifier public immutable verifier;
    uint128 public denomination;

    mapping(bytes32 => bool) public nullifierHashes;
    // we store all commitments just to prevent accidental deposits with the same commitment
    mapping(bytes32 => bool) public commitments;

    event Deposit(bytes32 indexed commitment, uint32 leafIndex, uint256 timestamp);
    event Withdrawal(address to, bytes32 nullifierHash, address indexed relayer, uint256 fee);

    /**
    @dev The constructor
    @param _verifier the address of SNARK verifier for this contract
    @param _hasher the address of MiMC hash contract
    @param _denomination transfer amount for each deposit
    @param _merkleTreeHeight the height of deposits' Merkle Tree
  */
    constructor(
        IVerifier _verifier,
        IHasher _hasher,
        uint128 _denomination,
        uint32 _merkleTreeHeight
    ) MerkleTreeWithHistory(_merkleTreeHeight, _hasher) {
        require(_denomination > 0, "denomination should be greater than 0");
        verifier = _verifier;
        denomination = _denomination;
    }

    /**
    @dev Deposit funds into the contract. The caller must send (for ETH) or approve (for ERC20) value equal to or `denomination` of this instance.
    @param _commit the note commitment, which is PedersenHash(nullifier + secret)
  */
    function deposit(bytes _commit) external payable {
        bytes32 _commitment = abi.decode(_commit, (bytes32));
        require(!commitments[_commitment], "The commitment has been submitted");

        uint32 insertedIndex = _insert(_commitment);
        commitments[_commitment] = true;
        _processDeposit();

        emit Deposit(_commitment, insertedIndex, block.timestamp);
    }

    /** @dev this function is defined in a child contract */
    function _processDeposit() internal virtual;

    /**
    @dev Withdraw a deposit from the contract. `proof` is a zkSNARK proof data, and input is an array of circuit public inputs
    `input` array consists of:
      - merkle root of all deposits in the contract
      - hash of unique deposit nullifier to prevent double spends
      - the recipient of funds
      - optional fee that goes to the transaction sender (usually a relay)
  */
    function withdraw(
        bytes calldata _proof,
        bytes _root_bytes,
        bytes _nullifierHash_bytes,
        address payable _recipient
    ) external payable {
        // We don't need these for our PoC
        address payable _relayer = address payable(0);
        uint256 _fee = 0;
        uint256 _refund = 0;
        // Polkadot 32 byte addresses are not necessarily within the field the SNARK uses.
        uint256 recipient_input = uint256(_recipient) % 21888242871839275222246405745257275088548364400416034343698204186575808495617;

        bytes32 _root = abi.decode(_root_bytes, (bytes32));
        bytes32 _nullifierHash = abi.decode(_nullifierHash_bytes, (bytes32));
        // require(_fee <= denomination, "Fee exceeds transfer value");
        require(!nullifierHashes[_nullifierHash], "The note has been already spent");
        require(isKnownRoot(_root), "Cannot find your merkle root"); // Make sure to use a recent one
        require(
            verifier.verifyProof(
                _proof,
                [uint256(_root), uint256(_nullifierHash), recipient_input, uint256(_relayer), _fee, _refund]
            ),
            "Invalid withdraw proof"
        );

        nullifierHashes[_nullifierHash] = true;
        _processWithdraw(_recipient, _relayer, 0, 0);
        emit Withdrawal(_recipient, _nullifierHash, _relayer, 0);
    }

    /** @dev this function is defined in a child contract */
    function _processWithdraw(
        address payable _recipient,
        address payable _relayer,
        uint128 _fee,
        uint128 _refund
    ) internal virtual;

    /** @dev whether a note is already spent */
    function isSpent(bytes32 _nullifierHash) public view returns (bool) {
        return nullifierHashes[_nullifierHash];
    }

    /** @dev whether an array of notes is already spent */
    function isSpentArray(bytes32[] calldata _nullifierHashes) external view returns (bool[] memory spent) {
        spent = new bool[](_nullifierHashes.length);
        for (uint256 i = 0; i < _nullifierHashes.length; i++) {
            if (isSpent(_nullifierHashes[i])) {
                spent[i] = true;
            }
        }
    }
}
