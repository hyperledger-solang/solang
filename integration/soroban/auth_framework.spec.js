import * as StellarSdk from '@stellar/stellar-sdk';
import { readFileSync } from 'fs';
import { expect } from 'chai';
import path from 'path';
import { fileURLToPath } from 'url';
import { call_contract_function, toSafeJson } from './test_helpers.js';
import { Server } from '@stellar/stellar-sdk/rpc';

const __filename = fileURLToPath(import.meta.url);
const dirname = path.dirname(__filename);
const server = new Server("https://soroban-testnet.stellar.org");

function readContractAddress(filename) {
  return readFileSync(path.join(dirname, '.stellar', 'contract-ids', filename), 'utf8').trim();
}

describe('Auth Framework', () => {
  let keypair, a, b, c, a_invalid;

  before(async () => {
    console.log('Setting up cross contract tests...');

    keypair = StellarSdk.Keypair.fromSecret(readFileSync('alice.txt', 'utf8').trim());
    a = new StellarSdk.Contract(readContractAddress('a.txt'));
    b = new StellarSdk.Contract(readContractAddress('b.txt'));
    c = new StellarSdk.Contract(readContractAddress('c.txt'));
    a_invalid = new StellarSdk.Contract(readContractAddress('a_invalid.txt'));
  });

  it('calls a', async () => {
    let values = [
      b.address().toScVal(),
      c.address().toScVal()
    ];
    let res = await call_contract_function("call_b", server, keypair, a, ...values);

    expect(res.status, `Call to 'a' contract failed: ${toSafeJson(res)}`).to.equal("SUCCESS");
    expect(res.returnValue, `Unexpected return value for 'a': ${toSafeJson(res)}`).to.equal(22n);
  });

  it('call fails with invalid `a` contract', async () => {
    let values = [
      b.address().toScVal(),
      c.address().toScVal()
    ];
    let res = await call_contract_function("call_b", server, keypair, a_invalid, ...values);

    expect(res.status).to.not.equal("SUCCESS");
    expect(
      res.error || toSafeJson(res),
      'Missing expected Soroban auth error message'
    ).to.include("recording authorization only] encountered authorization not tied to the root contract invocation for an address. Use `require_auth()` in the top invocation or enable non-root authorization.");
  });

});
