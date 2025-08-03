import * as StellarSdk from '@stellar/stellar-sdk';
import { readFileSync } from 'fs';
import { expect } from 'chai';
import path from 'path';
import { fileURLToPath } from 'url';
import { call_contract_function, toSafeJson } from './test_helpers.js';
import { Server } from '@stellar/stellar-sdk/rpc';

const __filename = fileURLToPath(import.meta.url);
const dirname = path.dirname(__filename);

describe('Runtime Error', () => {
  let keypair, contract;
  const server = new Server("https://soroban-testnet.stellar.org");

  before(async () => {
    console.log('Setting up runtime_error.sol contract tests...');

    // read secret from file
    const secret = readFileSync('alice.txt', 'utf8').trim();
    keypair = StellarSdk.Keypair.fromSecret(secret);

    const contractIdFile = path.join(dirname, '.stellar', 'contract-ids', 'Error.txt');
    const contractAddr = readFileSync(contractIdFile, 'utf8').trim();

    contract = new StellarSdk.Contract(contractAddr);

    // call decrement once (to reach error state on the next call)
    await call_contract_function("decrement", server, keypair, contract);
  });

  it('prints error', async () => {
    // decrement the counter again, expecting a runtime error
    const res = await call_contract_function("decrement", server, keypair, contract);

    expect(res.status).to.not.equal("SUCCESS");
    // The error message may be in res.error or a safe string version
    const errorString = res.error || toSafeJson(res);
    expect(errorString).to.contain('runtime_error: math overflow in runtime_error.sol:6:9-19');
  });

});
