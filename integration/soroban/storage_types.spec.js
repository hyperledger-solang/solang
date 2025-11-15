import * as StellarSdk from '@stellar/stellar-sdk';
import { readFileSync } from 'fs';
import { expect } from 'chai';
import path from 'path';
import { fileURLToPath } from 'url';
import { call_contract_function, toSafeJson } from './test_helpers.js';
import { Server } from '@stellar/stellar-sdk/rpc';

const __filename = fileURLToPath(import.meta.url);
const dirname = path.dirname(__filename);

describe('StorageTypes', () => {
  let keypair, contract;
  const server = new Server("https://soroban-testnet.stellar.org");

  before(async () => {
    console.log('Setting up storage_types contract tests...');

    const secret = readFileSync('alice.txt', 'utf8').trim();
    keypair = StellarSdk.Keypair.fromSecret(secret);

    const contractIdFile = path.join(dirname, '.stellar', 'contract-ids', 'storage_types.txt');
    const contractAddr = readFileSync(contractIdFile, 'utf8').trim();

    contract = new StellarSdk.Contract(contractAddr);
  });

  it('check initial values', async () => {
    // On public testnet, values may have been changed by previous runs; just verify they're valid
    let res = await call_contract_function("sesa", server, keypair, contract);
    expect(res.status, `sesa() call failed: ${toSafeJson(res)}`).to.equal("SUCCESS");
    expect(typeof res.returnValue).to.equal('bigint');
    expect(res.returnValue >= 0n, `sesa should be non-negative: ${toSafeJson(res)}`).to.be.true;

    res = await call_contract_function("sesa1", server, keypair, contract);
    expect(res.status, `sesa1() call failed: ${toSafeJson(res)}`).to.equal("SUCCESS");
    expect(typeof res.returnValue).to.equal('bigint');
    expect(res.returnValue >= 0n, `sesa1 should be non-negative: ${toSafeJson(res)}`).to.be.true;

    res = await call_contract_function("sesa2", server, keypair, contract);
    expect(res.status, `sesa2() call failed: ${toSafeJson(res)}`).to.equal("SUCCESS");
    expect(typeof res.returnValue).to.equal('bigint');
    expect(res.returnValue >= 0n, `sesa2 should be non-negative: ${toSafeJson(res)}`).to.be.true;

    res = await call_contract_function("sesa3", server, keypair, contract);
    expect(res.status, `sesa3() call failed: ${toSafeJson(res)}`).to.equal("SUCCESS");
    expect(typeof res.returnValue).to.equal('bigint');
    expect(res.returnValue >= 0n, `sesa3 should be non-negative: ${toSafeJson(res)}`).to.be.true;
  });

  it('increment values', async () => {
    // Get values before increment
    let sesaBefore = await call_contract_function("sesa", server, keypair, contract);
    let sesa1Before = await call_contract_function("sesa1", server, keypair, contract);
    let sesa2Before = await call_contract_function("sesa2", server, keypair, contract);
    let sesa3Before = await call_contract_function("sesa3", server, keypair, contract);
    expect(sesaBefore.status).to.equal("SUCCESS");
    expect(sesa1Before.status).to.equal("SUCCESS");
    expect(sesa2Before.status).to.equal("SUCCESS");
    expect(sesa3Before.status).to.equal("SUCCESS");

    const sesaBeforeValue = BigInt(sesaBefore.returnValue);
    const sesa1BeforeValue = BigInt(sesa1Before.returnValue);
    const sesa2BeforeValue = BigInt(sesa2Before.returnValue);
    const sesa3BeforeValue = BigInt(sesa3Before.returnValue);

    // Increment
    let incRes = await call_contract_function("inc", server, keypair, contract);
    expect(incRes.status, `inc() call failed: ${toSafeJson(incRes)}`).to.equal("SUCCESS");

    // Verify values increased by 1
    let res = await call_contract_function("sesa", server, keypair, contract);
    expect(res.status).to.equal("SUCCESS");
    expect(res.returnValue, `sesa should increase by 1: ${toSafeJson(res)}`).to.equal(sesaBeforeValue + 1n);

    res = await call_contract_function("sesa1", server, keypair, contract);
    expect(res.status).to.equal("SUCCESS");
    expect(res.returnValue, `sesa1 should increase by 1: ${toSafeJson(res)}`).to.equal(sesa1BeforeValue + 1n);

    res = await call_contract_function("sesa2", server, keypair, contract);
    expect(res.status).to.equal("SUCCESS");
    expect(res.returnValue, `sesa2 should increase by 1: ${toSafeJson(res)}`).to.equal(sesa2BeforeValue + 1n);

    res = await call_contract_function("sesa3", server, keypair, contract);
    expect(res.status).to.equal("SUCCESS");
    expect(res.returnValue, `sesa3 should increase by 1: ${toSafeJson(res)}`).to.equal(sesa3BeforeValue + 1n);
  });

  it('decrement values', async () => {
    // Get values before decrement
    let sesaBefore = await call_contract_function("sesa", server, keypair, contract);
    let sesa1Before = await call_contract_function("sesa1", server, keypair, contract);
    let sesa2Before = await call_contract_function("sesa2", server, keypair, contract);
    let sesa3Before = await call_contract_function("sesa3", server, keypair, contract);
    expect(sesaBefore.status).to.equal("SUCCESS");
    expect(sesa1Before.status).to.equal("SUCCESS");
    expect(sesa2Before.status).to.equal("SUCCESS");
    expect(sesa3Before.status).to.equal("SUCCESS");

    const sesaBeforeValue = BigInt(sesaBefore.returnValue);
    const sesa1BeforeValue = BigInt(sesa1Before.returnValue);
    const sesa2BeforeValue = BigInt(sesa2Before.returnValue);
    const sesa3BeforeValue = BigInt(sesa3Before.returnValue);

    // Decrement
    let decRes = await call_contract_function("dec", server, keypair, contract);
    expect(decRes.status, `dec() call failed: ${toSafeJson(decRes)}`).to.equal("SUCCESS");

    // Verify values decreased by 1 (but not below 0)
    let res = await call_contract_function("sesa", server, keypair, contract);
    expect(res.status).to.equal("SUCCESS");
    const expectedSesa = sesaBeforeValue > 0n ? sesaBeforeValue - 1n : 0n;
    expect(res.returnValue, `sesa should decrease by 1 (or stay at 0): ${toSafeJson(res)}`).to.equal(expectedSesa);

    res = await call_contract_function("sesa1", server, keypair, contract);
    expect(res.status).to.equal("SUCCESS");
    const expectedSesa1 = sesa1BeforeValue > 0n ? sesa1BeforeValue - 1n : 0n;
    expect(res.returnValue, `sesa1 should decrease by 1 (or stay at 0): ${toSafeJson(res)}`).to.equal(expectedSesa1);

    res = await call_contract_function("sesa2", server, keypair, contract);
    expect(res.status).to.equal("SUCCESS");
    const expectedSesa2 = sesa2BeforeValue > 0n ? sesa2BeforeValue - 1n : 0n;
    expect(res.returnValue, `sesa2 should decrease by 1 (or stay at 0): ${toSafeJson(res)}`).to.equal(expectedSesa2);

    res = await call_contract_function("sesa3", server, keypair, contract);
    expect(res.status).to.equal("SUCCESS");
    const expectedSesa3 = sesa3BeforeValue > 0n ? sesa3BeforeValue - 1n : 0n;
    expect(res.returnValue, `sesa3 should decrease by 1 (or stay at 0): ${toSafeJson(res)}`).to.equal(expectedSesa3);
  });
});

