// A very stripped down library version of the tornado core "cli.js"
//
// Source: https://github.com/tornadocash/tornado-core/blob/master/src/cli.js

const fs = require('fs')
const buildGroth16 = require('websnark/src/groth16')
const snarkjs = require('snarkjs')
const crypto = require('crypto')
const circomlib = require('circomlib')
const bigInt = snarkjs.bigInt
const merkleTree = require('fixed-merkle-tree')
const websnarkUtils = require('websnark/src/utils')

let circuit, proving_key, groth16, netId, MERKLE_TREE_HEIGHT

const PRIME_FIELD = 21888242871839275222246405745257275088548364400416034343698204186575808495617n;

/** Generate random number of specified byte length */
const rbigint = nbytes => snarkjs.bigInt.leBuff2int(crypto.randomBytes(nbytes));

/** Compute pedersen hash */
const pedersenHash = data => circomlib.babyJub.unpackPoint(circomlib.pedersenHash.hash(data))[0];

/** BigNumber to hex string of specified length */
export function toHex(number, length = 32) {
    const str = number instanceof Buffer ? number.toString('hex') : bigInt(number).toString(16)
    return '0x' + str.padStart(length * 2, '0')
}

// Wasm is little endian
function proofToLE(proof) {
    const segments = proof.slice(2).match(/.{1,64}/g);
    const swapped = segments.map(s => swapEndianness(s))
    return '0x' + swapped.join('')
}

function swapEndianness(hexString) {
    //const hexString = bigInt.toString(16); // Convert to hexadecimal string
    const paddedHexString = hexString.length % 2 !== 0 ? '0' + hexString : hexString; // Pad with zeros if needed
    const byteSegments = paddedHexString.match(/.{1,2}/g); // Split into two-character segments
    const reversedSegments = byteSegments.reverse(); // Reverse the order of segments
    const reversedHexString = reversedSegments.join(''); // Join segments back into a string
    return reversedHexString; // Parse the reversed string as a hexadecimal value
}

function parseNote(noteString) {
    const noteRegex = /tornado-(?<currency>\w+)-(?<amount>[\d.]+)-(?<netId>\d+)-0x(?<note>[0-9a-fA-F]{124})/g
    const match = noteRegex.exec(noteString)
    if (!match) {
        throw new Error('The note has invalid format')
    }

    const buf = Buffer.from(match.groups.note, 'hex')
    const nullifier = bigInt.leBuff2int(buf.slice(0, 31))
    const secret = bigInt.leBuff2int(buf.slice(31, 62))
    const deposit = createDeposit({ nullifier, secret })
    const netId = Number(match.groups.netId)

    return { currency: match.groups.currency, amount: match.groups.amount, netId, deposit }
}

export async function init_snark({ networkId = 43, merkle_tree_height = 20 }) {
    netId = networkId;
    MERKLE_TREE_HEIGHT = merkle_tree_height;
    circuit = require(__dirname + '/tornado-cli/build/circuits/withdraw.json');
    proving_key = fs.readFileSync(__dirname + '/tornado-cli/build/circuits/withdraw_proving_key.bin').buffer;
    groth16 = await buildGroth16();
}

function createDeposit({ nullifier, secret }) {
    const deposit = { nullifier, secret }
    deposit.preimage = Buffer.concat([deposit.nullifier.leInt2Buff(31), deposit.secret.leInt2Buff(31)]);
    deposit.commitment = pedersenHash(deposit.preimage);
    deposit.commitmentHex = toHex(deposit.commitment);
    deposit.nullifierHash = pedersenHash(deposit.nullifier.leInt2Buff(31));
    deposit.nullifierHex = toHex(deposit.nullifierHash);
    return deposit;
}

export function createNote({ currency = 'ETH', amount = 1000000000000 }) {
    const deposit = createDeposit({ nullifier: rbigint(31), secret: rbigint(31) });
    const note = toHex(deposit.preimage, 62);
    const noteString = `tornado-${currency}-${amount}-${netId}-${note}`;
    // console.log(`Your commitment: ${toHex(deposit.commitment, 32)}`); // Uncomment for debug
    return { noteString, commitment: toHex(deposit.commitment) };
}

// The 'leaves' argument is supposed to be a list of commitments sorted by their leafIndex (chronologically sorted)
async function generateMerkleProof(deposit, leafIndex, leaves) {
    console.log('generating merkle proof');
    let tree = new merkleTree(MERKLE_TREE_HEIGHT, leaves);
    const { pathElements, pathIndices } = tree.path(leafIndex);
    return { pathElements, pathIndices, root: tree.root() }
}

async function generateProof({ deposit, recipient, leaves }) {
    const leafIndex = leaves.indexOf(toHex(deposit.commitment));
    const { root, pathElements, pathIndices } = await generateMerkleProof(deposit, leafIndex, leaves);

    const input = {
        // Public snark inputs
        root: root,
        nullifierHash: deposit.nullifierHash,
        recipient: bigInt(recipient),
        relayer: bigInt(0),
        fee: bigInt(0),
        refund: bigInt(0),

        // Private snark inputs
        nullifier: deposit.nullifier,
        secret: deposit.secret,
        pathElements: pathElements,
        pathIndices: pathIndices,
    }

    console.log('Generating SNARK proof');
    console.time('Proof time');
    const proofData = await websnarkUtils.genWitnessAndProve(groth16, input, circuit, proving_key);
    const { proof } = websnarkUtils.toSolidityInput(proofData);
    console.timeEnd('Proof time');

    const args = [
        toHex(input.root),
        toHex(input.nullifierHash),
        toHex(input.recipient),
        toHex(input.relayer, 20),
        toHex(input.fee),
        toHex(input.refund),
    ]

    // console.log(args); // uncomment for debug
    return { proof: proofToLE(proof), args }
}

export async function withdraw(to, noteString, leaves) {
    // Substrate 32 byte addrs aren't necessarily within the finite field (as opposed to ETH addresses).
    // This hack naturally makes it work regardless. Maybe it would even be fine in production too.
    const recipient = to % PRIME_FIELD;
    const parsed_note = parseNote(noteString);
    return await generateProof({ deposit: parsed_note.deposit, recipient, leaves });
}
