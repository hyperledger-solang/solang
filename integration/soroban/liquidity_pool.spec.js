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

describe('Liquidity Pool', () => {
  let keypair;
  let owner;
  let pool;
  let tokenA;
  let tokenB;

  before(async () => {
    keypair = StellarSdk.Keypair.fromSecret(readFileSync('alice.txt', 'utf8').trim());
    owner = new StellarSdk.Address(keypair.publicKey()).toScVal();

    pool = new StellarSdk.Contract(readContractAddress('liquidity_pool.txt'));
    tokenA = new StellarSdk.Contract(readContractAddress('liquidity_pool_token_a.txt'));
    tokenB = new StellarSdk.Contract(readContractAddress('liquidity_pool_token_b.txt'));
  });

  it('supports deposit, swap, and withdraw', async () => {
    let res = await call_contract_function('mint', server, keypair, tokenA, owner, u64(100_000));
    expect(res.status, `mint token A failed: ${toSafeJson(res)}`).to.equal('SUCCESS');

    res = await call_contract_function('mint', server, keypair, tokenB, owner, u64(100_000));
    expect(res.status, `mint token B failed: ${toSafeJson(res)}`).to.equal('SUCCESS');

    res = await call_contract_function(
      'deposit',
      server,
      keypair,
      pool,
      owner,
      tokenA.address().toScVal(),
      tokenB.address().toScVal(),
      u64(10_000),
      u64(9_000),
      u64(20_000),
      u64(18_000),
    );
    expect(res.status, `deposit failed: ${toSafeJson(res)}`).to.equal('SUCCESS');

    res = await call_contract_view('reserve_a', server, keypair, pool);
    expect(res.status, `reserve_a failed: ${toSafeJson(res)}`).to.equal('SUCCESS');
    expect(res.returnValue).to.equal(10_000n);

    res = await call_contract_view('reserve_b', server, keypair, pool);
    expect(res.status, `reserve_b failed: ${toSafeJson(res)}`).to.equal('SUCCESS');
    expect(res.returnValue).to.equal(20_000n);

    res = await call_contract_view('balance_shares', server, keypair, pool, owner);
    expect(res.status, `balance_shares failed: ${toSafeJson(res)}`).to.equal('SUCCESS');
    expect(res.returnValue).to.equal(14_142n);

    res = await call_contract_function(
      'swap_buy_a',
      server,
      keypair,
      pool,
      owner,
      tokenA.address().toScVal(),
      tokenB.address().toScVal(),
      u64(1_000),
      u64(3_000),
    );
    expect(res.status, `swap_buy_a failed: ${toSafeJson(res)}`).to.equal('SUCCESS');

    res = await call_contract_view('reserve_a', server, keypair, pool);
    expect(res.status, `reserve_a after swap failed: ${toSafeJson(res)}`).to.equal('SUCCESS');
    expect(res.returnValue).to.equal(9_000n);

    res = await call_contract_view('reserve_b', server, keypair, pool);
    expect(res.status, `reserve_b after swap failed: ${toSafeJson(res)}`).to.equal('SUCCESS');
    expect(res.returnValue).to.equal(22_229n);

    res = await call_contract_function(
      'withdraw',
      server,
      keypair,
      pool,
      owner,
      tokenA.address().toScVal(),
      tokenB.address().toScVal(),
      u64(7_071),
      u64(0),
      u64(0),
    );
    expect(res.status, `withdraw failed: ${toSafeJson(res)}`).to.equal('SUCCESS');

    res = await call_contract_view('balance_shares', server, keypair, pool, owner);
    expect(res.status, `balance_shares after withdraw failed: ${toSafeJson(res)}`).to.equal('SUCCESS');
    expect(res.returnValue).to.equal(7_071n);

    res = await call_contract_view('reserve_a', server, keypair, pool);
    expect(res.status, `reserve_a after withdraw failed: ${toSafeJson(res)}`).to.equal('SUCCESS');
    expect(res.returnValue).to.equal(4_500n);

    res = await call_contract_view('reserve_b', server, keypair, pool);
    expect(res.status, `reserve_b after withdraw failed: ${toSafeJson(res)}`).to.equal('SUCCESS');
    expect(res.returnValue).to.equal(11_115n);
  });
});
