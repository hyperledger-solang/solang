import * as StellarSdk from '@stellar/stellar-sdk';
import { readFileSync } from 'fs';
import { expect } from 'chai';
import path from 'path';
import { fileURLToPath } from 'url';
import {
  call_contract_function,
  extractLogMessagesFromDiagnosticEvents,
  toSafeJson,
} from './test_helpers.js';
import { Server } from '@stellar/stellar-sdk/rpc';

const __filename = fileURLToPath(import.meta.url);
const dirname = path.dirname(__filename);
const server = new Server("https://soroban-testnet.stellar.org");

function readContractAddress(filename) {
  return readFileSync(path.join(dirname, '.stellar', 'contract-ids', filename), 'utf8').trim();
}

describe('Cross Contract Calls', () => {
  let keypair, caller, callee, calleeRust;

  before(async () => {
    console.log('Setting up cross contract tests...');

    keypair = StellarSdk.Keypair.fromSecret(readFileSync('alice.txt', 'utf8').trim());
    caller = new StellarSdk.Contract(readContractAddress('caller.txt'));
    callee = new StellarSdk.Contract(readContractAddress('callee.txt'));
    calleeRust = new StellarSdk.Contract(readContractAddress('hello_world.txt'));
  });

  it('calls Rust contract', async () => {
    let addr = calleeRust.address().toScVal();
    let values = [
      new StellarSdk.xdr.Uint64(BigInt(1)),
      new StellarSdk.xdr.Uint64(BigInt(2)),
      new StellarSdk.xdr.Uint64(BigInt(0))
    ].map(StellarSdk.xdr.ScVal.scvU64);

    let res = await call_contract_function("add", server, keypair, caller, addr, ...values);

    expect(res.status, `Rust contract tx failed: ${toSafeJson(res)}`).to.equal("SUCCESS");
    // Return value is already decoded (should be 3n for u64 add)
    expect(res.returnValue, `Unexpected returnValue for Rust contract: ${toSafeJson(res)}`).to.equal(3n);

    const logMessages = extractLogMessagesFromDiagnosticEvents(res.raw);
    expect(logMessages.length > 0, "No logMessages found in Rust contract response").to.be.true;
    expect(logMessages[0]).to.contain('Soroban SDK add function called!');
  });

  it('calls Solidity contract', async () => {
    let addr = callee.address().toScVal();
    let values = [
      new StellarSdk.xdr.Uint64(BigInt(1)),
      new StellarSdk.xdr.Uint64(BigInt(2)),
      new StellarSdk.xdr.Uint64(BigInt(0))
    ].map(StellarSdk.xdr.ScVal.scvU64);

    let res = await call_contract_function("add", server, keypair, caller, addr, ...values);

    expect(res.status, `Solidity contract tx failed: ${toSafeJson(res)}`).to.equal("SUCCESS");
    expect(res.returnValue, `Unexpected returnValue for Solidity contract: ${toSafeJson(res)}`).to.equal(3n);

    const logMessages = extractLogMessagesFromDiagnosticEvents(res.raw);
    expect(logMessages.length > 0, "No logMessages found in Solidity contract response").to.be.true;
    expect(logMessages[0]).to.contain('add called in Solidity');
  });
});
