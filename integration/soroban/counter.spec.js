import * as StellarSdk from '@stellar/stellar-sdk';
import { readFileSync } from 'fs';
import { expect } from 'chai';
import path from 'path';
import { fileURLToPath } from 'url';
import { call_contract_function, toSafeJson } from './test_helpers.js';
import { Server } from '@stellar/stellar-sdk/rpc';

const __filename = fileURLToPath(import.meta.url);
const dirname = path.dirname(__filename);

describe('Counter', () => {
  let keypair, contract;
  const server = new Server("https://soroban-testnet.stellar.org");

  before(async () => {
    console.log('Setting up counter contract tests...');

    // read secret from file
    const secret = readFileSync('alice.txt', 'utf8').trim();
    keypair = StellarSdk.Keypair.fromSecret(secret);

    let contractIdFile = path.join(dirname, '.stellar', 'contract-ids', 'counter.txt');
    // read contract address from file
    const contractAddr = readFileSync(contractIdFile, 'utf8').trim();
    // load contract
    contract = new StellarSdk.Contract(contractAddr);
  });

  it('get initial counter', async () => {
    let res = await call_contract_function("count", server, keypair, contract);

    expect(res.status, `Counter 'count' call failed: ${toSafeJson(res)}`).to.equal("SUCCESS");
    // On public testnet, the value may have been changed by previous runs; just assert it's a bigint >= 0
    expect(typeof res.returnValue).to.equal('bigint');
    expect(res.returnValue >= 0n, `Counter should be non-negative: ${toSafeJson(res)}`).to.be.true;
  });

  it('increment counter', async () => {
    // get current value first (network may have prior state)
    let before = await call_contract_function("count", server, keypair, contract);
    expect(before.status, `Counter 'count' before increment failed: ${toSafeJson(before)}`).to.equal("SUCCESS");
    const expected = BigInt(before.returnValue) + 1n;

    // increment the counter
    let incRes = await call_contract_function("increment", server, keypair, contract);
    expect(incRes.status, `Counter 'increment' call failed: ${toSafeJson(incRes)}`).to.equal("SUCCESS");

    // get the count again
    let after = await call_contract_function("count", server, keypair, contract);
    expect(after.status, `Counter 'count' after increment failed: ${toSafeJson(after)}`).to.equal("SUCCESS");
    expect(after.returnValue, `Unexpected counter value after increment: ${toSafeJson(after)}`).to.equal(expected);
  });
});
