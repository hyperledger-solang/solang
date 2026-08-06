import * as StellarSdk from '@stellar/stellar-sdk';
import { readFileSync } from 'fs';
import { expect } from 'chai';
import path from 'path';
import { fileURLToPath } from 'url';
import { call_contract_function, call_contract_view, toSafeJson } from './test_helpers.js';
import { Server } from '@stellar/stellar-sdk/rpc';

const __filename = fileURLToPath(import.meta.url);
const dirname = path.dirname(__filename);
const server = new Server('https://soroban-testnet.stellar.org');

function readContractAddress(filename) {
  return readFileSync(path.join(dirname, '.stellar', 'contract-ids', filename), 'utf8').trim();
}

function u64(value) {
  return StellarSdk.xdr.ScVal.scvU64(new StellarSdk.xdr.Uint64(BigInt(value)));
}

function randomAddress() {
  return new StellarSdk.Address(StellarSdk.Keypair.random().publicKey()).toScVal();
}

describe('Mint lock', () => {
  let keypair;
  let mintLock;
  let token;

  before(async () => {
    keypair = StellarSdk.Keypair.fromSecret(readFileSync('alice.txt', 'utf8').trim());
    mintLock = new StellarSdk.Contract(readContractAddress('mint_lock.txt'));
    token = new StellarSdk.Contract(readContractAddress('ml_token.txt'));
  });

  async function balance(owner) {
    const res = await call_contract_view('balance', server, keypair, token, owner);
    expect(res.status, `balance failed: ${toSafeJson(res)}`).to.equal('SUCCESS');
    return res.returnValue;
  }

  it('mints up to the configured limit and blocks the rest', async () => {
    // Fresh minter/user each run so persisted `minted`/balances stay deterministic.
    const minter = randomAddress();
    const user = randomAddress();
    const tokenAddr = token.address().toScVal();

    let res = await call_contract_function('set_limit', server, keypair, mintLock, minter, u64(100));
    expect(res.status, `set_limit failed: ${toSafeJson(res)}`).to.equal('SUCCESS');

    // First mint of 60 is within the 100 limit.
    res = await call_contract_function('mint', server, keypair, mintLock, tokenAddr, minter, user, u64(60));
    expect(res.status, `first mint failed: ${toSafeJson(res)}`).to.equal('SUCCESS');
    expect(await balance(user)).to.equal(60n);

    // Second mint of 60 would total 120 > 100: rejected.
    res = await call_contract_function('mint', server, keypair, mintLock, tokenAddr, minter, user, u64(60));
    expect(res.status, `over-limit mint unexpectedly succeeded: ${toSafeJson(res)}`).to.not.equal('SUCCESS');

    // Only the first mint went through.
    expect(await balance(user)).to.equal(60n);
  });
});
