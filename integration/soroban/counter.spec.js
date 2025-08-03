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

  it('get correct initial counter', async () => {
    let res = await call_contract_function("count", server, keypair, contract);

    expect(res.status, `Counter 'count' call failed: ${toSafeJson(res)}`).to.equal("SUCCESS");
    expect(res.returnValue, `Unexpected counter value: ${toSafeJson(res)}`).to.equal(10n);
  });

  it('increment counter', async () => {
    // increment the counter
    let incRes = await call_contract_function("increment", server, keypair, contract);
    expect(incRes.status, `Counter 'increment' call failed: ${toSafeJson(incRes)}`).to.equal("SUCCESS");

    // get the count again
    let res = await call_contract_function("count", server, keypair, contract);
    expect(res.status, `Counter 'count' after increment failed: ${toSafeJson(res)}`).to.equal("SUCCESS");
    expect(res.returnValue, `Unexpected counter value after increment: ${toSafeJson(res)}`).to.equal(11n);
  });
});
