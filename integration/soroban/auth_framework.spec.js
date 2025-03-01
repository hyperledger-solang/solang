import * as StellarSdk from '@stellar/stellar-sdk';
import { readFileSync } from 'fs';
import { expect } from 'chai';
import path from 'path';
import { fileURLToPath } from 'url';
import { call_contract_function, extractLogEvent } from './test_helpers.js';
import { assert } from 'console';

const __filename = fileURLToPath(import.meta.url);
const dirname = path.dirname(__filename);
const server = new StellarSdk.SorobanRpc.Server("https://soroban-testnet.stellar.org:443");

function readContractAddress(filename) {
  return readFileSync(path.join(dirname, '.soroban', 'contract-ids', filename), 'utf8').trim();
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


    expect(res.returnValue().value().toString()).to.equal("22");
    
  });

  it ('call falis with invalid `a` contract', async () => {
    
    
    let values = [
        b.address().toScVal(),
        c.address().toScVal()
    ];

    let res = await call_contract_function("call_b", server, keypair, a_invalid, ...values);

    assert(res.toString().includes("recording authorization only] encountered authorization not tied to the root contract invocation for an address. Use `require_auth()` in the top invocation or enable non-root authorization."));

  });


});
