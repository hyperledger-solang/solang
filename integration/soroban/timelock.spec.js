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

function u32(value) {
  return StellarSdk.xdr.ScVal.scvU32(value);
}

function u64(value) {
  return StellarSdk.xdr.ScVal.scvU64(new StellarSdk.xdr.Uint64(BigInt(value)));
}

describe('Timelock', () => {
  let keypair;
  let owner;
  let timelock;
  let token;

  before(async () => {
    keypair = StellarSdk.Keypair.fromSecret(readFileSync('alice.txt', 'utf8').trim());
    owner = new StellarSdk.Address(keypair.publicKey()).toScVal();

    timelock = new StellarSdk.Contract(readContractAddress('timelock.txt'));
    token = new StellarSdk.Contract(readContractAddress('timelock_token.txt'));
  });

  async function mint(contract, to, amount) {
    const res = await call_contract_function('mint', server, keypair, contract, to, u64(amount));
    expect(res.status, `mint failed: ${toSafeJson(res)}`).to.equal('SUCCESS');
  }

  async function balance(contract, ownerAddress) {
    const res = await call_contract_view('balance', server, keypair, contract, ownerAddress);
    expect(res.status, `balance failed: ${toSafeJson(res)}`).to.equal('SUCCESS');
    return res.returnValue;
  }

  async function currentTimestamp() {
    const res = await call_contract_view('now_ts', server, keypair, timelock);
    expect(res.status, `now_ts failed: ${toSafeJson(res)}`).to.equal('SUCCESS');
    return BigInt(res.returnValue);
  }

  async function timelockState() {
    const res = await call_contract_view('state', server, keypair, timelock);
    expect(res.status, `state failed: ${toSafeJson(res)}`).to.equal('SUCCESS');
    return Number(res.returnValue);
  }

  it('rejects claim before bound for TimeBoundKind.After', async () => {
    const timelockAddress = timelock.address().toScVal();
    const initialTimestamp = await currentTimestamp();
    const boundTimestamp = initialTimestamp + 3_600n;

    await mint(token, owner, 500);

    let res = await call_contract_function(
      'deposit',
      server,
      keypair,
      timelock,
      owner,
      token.address().toScVal(),
      u64(200),
      u32(1),
      u64(boundTimestamp),
    );
    expect(res.status, `deposit failed: ${toSafeJson(res)}`).to.equal('SUCCESS');

    expect(await timelockState()).to.equal(1);
    expect(await balance(token, owner)).to.equal(300n);
    expect(await balance(token, timelockAddress)).to.equal(200n);

    res = await call_contract_function(
      'claim',
      server,
      keypair,
      timelock,
      token.address().toScVal(),
      owner,
    );
    expect(res.status, `early claim unexpectedly succeeded: ${toSafeJson(res)}`).to.not.equal(
      'SUCCESS',
    );

    expect(await timelockState()).to.equal(1);
    expect(await balance(token, owner)).to.equal(300n);
    expect(await balance(token, timelockAddress)).to.equal(200n);
  });
});
