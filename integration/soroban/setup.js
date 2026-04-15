
import 'dotenv/config';
import { mkdirSync, readdirSync, readFileSync, writeFileSync, existsSync } from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';
import crypto from 'crypto';
import {
  Keypair,
  Address,
  TransactionBuilder,
  BASE_FEE,
  Networks,
  Operation,
  rpc,
  xdr,
  StrKey,
} from '@stellar/stellar-sdk';

console.log('###################### Initializing (SDK) ########################');

const __filename = fileURLToPath(import.meta.url);
const dirname = path.dirname(__filename);

// --- Network config (mirrors your CLI) ---
const RPC_URL = process.env.SOROBAN_RPC_URL || 'https://soroban-testnet.stellar.org';
const NETWORK_PASSPHRASE = Networks.TESTNET;

// --- Paths ---
const CONTRACT_IDS_DIR = path.join(dirname, '.stellar', 'contract-ids');
const SIGNER_FILES = {
  alice: path.join(dirname, 'alice.txt'),
  bob: path.join(dirname, 'bob.txt'),
  charlie: path.join(dirname, 'charlie.txt'),
};

// --- SDK server ---
const server = new rpc.Server(RPC_URL);

// ---------- helpers ----------
const filenameNoExtension = (filename) => path.basename(filename, path.extname(filename));

function logStep(s) {
  console.log(`\n=== ${s} ===`);
}

// Extract a valid Ed25519 seed ("S..." StrKey) from any string; return null if not found
function extractSeed(raw) {
  if (!raw) return null;
  const text = String(raw).trim();

  // 1) Common "secret: S..." format
  const line = text.match(/^secret:\s*(\S+)/mi)?.[1];
  if (line && line.startsWith('S')) return line;

  // 2) Look for any S... seed inside the text (base32 chars, total length 56)
  const m = text.match(/\bS[ABCDEFGHIJKLMNOPQRSTUVWXYZ234567]{55}\b/);
  if (m) return m[0];

  // 3) Maybe the whole file/env is just the seed
  if (text.startsWith('S') && text.length >= 56) return text.split(/\s+/)[0];

  return null;
}

// Save signer seed in the legacy format expected by tests.
function saveSignerSeedOnly(signerName, kp) {
  const signerFile = SIGNER_FILES[signerName];
  if (!signerFile) {
    throw new Error(`unknown signer '${signerName}'`);
  }
  writeFileSync(signerFile, kp.secret().trim() + '\n');
}

// create/fund or reuse signer account
async function getSigner(signerName) {
  const signerFile = SIGNER_FILES[signerName];
  if (!signerFile) {
    throw new Error(`unknown signer '${signerName}'`);
  }

  const envVarName = `${signerName.toUpperCase()}_SECRET`;
  const envRaw = process.env[envVarName]?.trim();
  if (envRaw) {
    const seed = extractSeed(envRaw);
    if (!seed) throw new Error(`${envVarName} is set but not a valid S… seed`);
    const kp = Keypair.fromSecret(seed);
    await server.requestAirdrop(kp.publicKey()).catch(() => {}); // no-op if already funded
    saveSignerSeedOnly(signerName, kp); // normalize file for tests
    return kp;
  }

  // if signer file exists, parse/normalize it (supports multi-line legacy)
  if (existsSync(signerFile)) {
    const raw = readFileSync(signerFile, 'utf8');
    const seed = extractSeed(raw);
    if (seed) {
      const kp = Keypair.fromSecret(seed);
      await server.requestAirdrop(kp.publicKey()).catch(() => {});
      // normalize file to seed-only so future runs & tests are stable
      saveSignerSeedOnly(signerName, kp);
      return kp;
    }
    // fall through if file was malformed
  }

  // otherwise generate & fund
  const kp = Keypair.random();
  logStep(`Funding ${signerName} (${kp.publicKey()}) via Friendbot`);
  await server.requestAirdrop(kp.publicKey());
  saveSignerSeedOnly(signerName, kp);
  return kp;
}

async function loadSourceAccount(publicKey) {
  // For Soroban you fetch sequence via RPC:
  return server.getAccount(publicKey);
}

// Upload a WASM module (on-chain code). We also compute its SHA-256 (wasmHash) locally.
async function uploadWasm(sourceAccount, signer, wasmBytes) {
  const tx = new TransactionBuilder(sourceAccount, {
    fee: BASE_FEE,
    networkPassphrase: NETWORK_PASSPHRASE,
  })
    .addOperation(Operation.uploadContractWasm({ wasm: wasmBytes }))
    .setTimeout(60)
    .build();

  // prepare (simulate adds resources/footprint), sign, send
  const prepared = await server.prepareTransaction(tx);
  prepared.sign(signer);
  const sent = await server.sendTransaction(prepared);
  await server.pollTransaction(sent.hash);

  // The wasmHash is the SHA-256 of the bytes; createContract expects this hash.
  const wasmHash = crypto.createHash('sha256').update(wasmBytes).digest(); // Buffer(32)
  return wasmHash;
}

// Extract the simulation return value (ScVal), supporting both parsed and base64 shapes
function extractSimRetval(sim) {
  const candidate = sim?.result?.retval ?? sim?.results?.[0]?.retval;
  if (!candidate) return null;

  // Parsed object (xdr.ScVal): has a .switch() function (and often .toXDR())
  if (candidate && typeof candidate.switch === 'function') return candidate;

  // Base64-encoded XDR string (older shapes)
  if (typeof candidate === 'string') return xdr.ScVal.fromXDR(candidate, 'base64');

  // xdr object with toXDR method (rare edge)
  if (candidate && typeof candidate.toXDR === 'function') return candidate;

  return null;
}

// Create a contract instance from the uploaded wasmHash.
// Returns the "C..." contract id using simulation (no event parsing).
async function createContract(sourceAccount, signer, wasmHash) {
  const deployer = new Address(signer.publicKey());
  const salt = crypto.randomBytes(32); // deterministic ID for this deployer+salt

  // Build the tx (not prepared yet)
  let createTx = new TransactionBuilder(sourceAccount, {
    fee: BASE_FEE,
    networkPassphrase: NETWORK_PASSPHRASE,
  })
    .addOperation(
      Operation.createCustomContract({
        address: deployer,
        wasmHash,            // sha256(wasm bytes)
        constructorArgs: [], // add args here if your contract has an init
        salt,                // deterministic contract id
      })
    )
    .setTimeout(60)
    .build();

  // 1) SIMULATE to read the return value (contract address) before submitting
  const sim = await server.simulateTransaction(createTx);
  const scv = extractSimRetval(sim);
  if (!scv) {
    throw new Error(
      `simulateTransaction returned no retval for createCustomContract: ${JSON.stringify(sim)}`
    );
  }
  if (scv.switch() !== xdr.ScValType.scvAddress()) {
    throw new Error('createCustomContract retval is not an Address ScVal');
  }
  const scAddr = scv.address();
  if (scAddr.switch() !== xdr.ScAddressType.scAddressTypeContract()) {
    throw new Error('createCustomContract retval Address is not a contract');
  }
  const contractId = StrKey.encodeContract(scAddr.contractId()); // => "C..."

  // 2) Prepare, sign, send, poll
  createTx = await server.prepareTransaction(createTx);
  createTx.sign(signer);
  const sent = await server.sendTransaction(createTx);
  await server.pollTransaction(sent.hash);

  return contractId;
}

async function deployOne(wasmPath, signerName, signer) {
  const name = filenameNoExtension(wasmPath);
  const outFile = path.join(CONTRACT_IDS_DIR, `${name}.txt`);
  const wasmBytes = readFileSync(wasmPath);

  logStep(`[${signerName}] Uploading WASM: ${wasmPath}`);
  let account = await loadSourceAccount(signer.publicKey());
  const wasmHash = await uploadWasm(account, signer, wasmBytes);

  logStep(`[${signerName}] Creating contract for: ${name}`);
  account = await loadSourceAccount(signer.publicKey()); // refresh sequence
  const contractId = await createContract(account, signer, wasmHash);

  mkdirSync(CONTRACT_IDS_DIR, { recursive: true });
  writeFileSync(outFile, contractId + '\n');
  console.log(`✔ [${signerName}] Wrote contract id -> ${outFile}`);
}

async function deployAll() {
  const signerNames = ['alice', 'bob', 'charlie'];
  const signers = await Promise.all(
    signerNames.map(async (name) => ({ name, keypair: await getSigner(name) }))
  );
  const files = readdirSync(dirname).filter((f) => f.endsWith('.wasm'));

  // include your Rust artifact, same path you used before
  const rustWasm = path.join(
    'rust',
    'target',
    'wasm32v1-none',
    'release-with-logs',
    'hello_world.wasm'
  );
  if (!files.includes(rustWasm)) files.push(rustWasm);

  files.sort();
  console.log('Found WASM files:', files);

  // Shard files round-robin across signers; each signer runs sequentially to
  // keep account sequence numbers valid, while signer groups run in parallel.
  const shardMap = signers.map((s) => ({ signer: s, files: [] }));
  files.forEach((file, index) => {
    shardMap[index % shardMap.length].files.push(file);
  });

  await Promise.all(
    shardMap.map(async ({ signer, files: signerFiles }) => {
      for (const f of signerFiles) {
        const full = path.join(dirname, f);
        await deployOne(full, signer.name, signer.keypair);
      }
    })
  );

  console.log('Deployment shard summary:');
  for (const { signer, files: signerFiles } of shardMap) {
    console.log(`  ${signer.name}: ${signerFiles.length} contracts`);
  }
}

(async function main() {
  logStep('Network');
  console.log('RPC:', RPC_URL);
  console.log('Passphrase:', NETWORK_PASSPHRASE);

  await deployAll();
})().catch((e) => {
  console.error('\nDeployment failed:', e?.response ?? e);
  process.exit(1);
});
