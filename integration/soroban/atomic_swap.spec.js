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

describe('Atomic Swap', () => {
  let keypair;
  let swap;
  let tokenA;
  let tokenB;

  before(async () => {
    keypair = StellarSdk.Keypair.fromSecret(readFileSync('alice.txt', 'utf8').trim());
    swap = new StellarSdk.Contract(readContractAddress('atomic_swap.txt'));
    tokenA = new StellarSdk.Contract(readContractAddress('atomic_swap_token_a.txt'));
    tokenB = new StellarSdk.Contract(readContractAddress('atomic_swap_token_b.txt'));
  });

  async function mint(contract, owner, amount) {
    const res = await call_contract_function(
      'mint',
      server,
      keypair,
      contract,
      owner,
      u64(amount),
    );

    expect(
      res.status,
      `mint failed for ${contract.address().toString()}: ${toSafeJson(res)}`,
    ).to.equal('SUCCESS');
  }

  async function balance(contract, owner) {
    const res = await call_contract_view('balance', server, keypair, contract, owner);
    expect(
      res.status,
      `balance failed for ${contract.address().toString()}: ${toSafeJson(res)}`,
    ).to.equal('SUCCESS');
    return res.returnValue;
  }

  it('settles swap and refunds remainder to each party', async () => {
    const partyA = new StellarSdk.Address(StellarSdk.Keypair.random().publicKey()).toScVal();
    const partyB = new StellarSdk.Address(StellarSdk.Keypair.random().publicKey()).toScVal();
    const swapAddress = swap.address().toScVal();

    await mint(tokenA, partyA, 100);
    await mint(tokenB, partyB, 80);

    const res = await call_contract_function(
      'swap',
      server,
      keypair,
      swap,
      partyA,
      partyB,
      tokenA.address().toScVal(),
      tokenB.address().toScVal(),
      u64(40),
      u64(30),
      u64(50),
      u64(35),
    );

    expect(res.status, `swap failed: ${toSafeJson(res)}`).to.equal('SUCCESS');

    expect(await balance(tokenA, partyA)).to.equal(65n);
    expect(await balance(tokenA, partyB)).to.equal(35n);
    expect(await balance(tokenA, swapAddress)).to.equal(0n);

    expect(await balance(tokenB, partyA)).to.equal(30n);
    expect(await balance(tokenB, partyB)).to.equal(50n);
    expect(await balance(tokenB, swapAddress)).to.equal(0n);
  });

  it('reverts when minimum requested price is not met', async () => {
    const partyA = new StellarSdk.Address(StellarSdk.Keypair.random().publicKey()).toScVal();
    const partyB = new StellarSdk.Address(StellarSdk.Keypair.random().publicKey()).toScVal();
    const swapAddress = swap.address().toScVal();

    await mint(tokenA, partyA, 100);
    await mint(tokenB, partyB, 80);

    const res = await call_contract_function(
      'swap',
      server,
      keypair,
      swap,
      partyA,
      partyB,
      tokenA.address().toScVal(),
      tokenB.address().toScVal(),
      u64(40),
      u64(60),
      u64(50),
      u64(35),
    );

    expect(res.status, `swap unexpectedly succeeded: ${toSafeJson(res)}`).to.not.equal('SUCCESS');

    expect(await balance(tokenA, partyA)).to.equal(100n);
    expect(await balance(tokenA, partyB)).to.equal(0n);
    expect(await balance(tokenA, swapAddress)).to.equal(0n);

    expect(await balance(tokenB, partyA)).to.equal(0n);
    expect(await balance(tokenB, partyB)).to.equal(80n);
    expect(await balance(tokenB, swapAddress)).to.equal(0n);
  });
});
