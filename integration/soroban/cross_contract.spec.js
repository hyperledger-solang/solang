import * as StellarSdk from '@stellar/stellar-sdk';
import { readFileSync } from 'fs';
import { expect } from 'chai';
import path from 'path';
import { fileURLToPath } from 'url';
import { call_contract_function, extractLogEvent } from './test_helpers.js';

const __filename = fileURLToPath(import.meta.url);
const dirname = path.dirname(__filename);
const server = new StellarSdk.SorobanRpc.Server("https://soroban-testnet.stellar.org:443");

function readContractAddress(filename) {
  return readFileSync(path.join(dirname, '.soroban', 'contract-ids', filename), 'utf8').trim();
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
    let returnValue = res.returnValue().value().toString();

    console.log(returnValue);
    expect(returnValue).to.equal("3");

    let logMessages = extractLogEvent(res.diagnosticEvents()).logMessages;
    console.log(logMessages);
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
    let returnValue = res.returnValue().value().toString();

    console.log(returnValue);
    expect(returnValue).to.equal("3");

    let logMessages = extractLogEvent(res.diagnosticEvents()).logMessages;
    console.log(logMessages);
    expect(logMessages[0]).to.contain('add called in Solidity');
  });
});
