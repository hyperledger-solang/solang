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
    let res = await call_contract_function("sesa", server, keypair, contract);
    expect(res.status, `sesa() call failed: ${toSafeJson(res)}`).to.equal("SUCCESS");
    expect(res.returnValue, `unexpected sesa: ${toSafeJson(res)}`).to.equal(1n);

    res = await call_contract_function("sesa1", server, keypair, contract);
    expect(res.status, `sesa1() call failed: ${toSafeJson(res)}`).to.equal("SUCCESS");
    expect(res.returnValue, `unexpected sesa1: ${toSafeJson(res)}`).to.equal(1n);

    res = await call_contract_function("sesa2", server, keypair, contract);
    expect(res.status, `sesa2() call failed: ${toSafeJson(res)}`).to.equal("SUCCESS");
    expect(res.returnValue, `unexpected sesa2: ${toSafeJson(res)}`).to.equal(2n);

    res = await call_contract_function("sesa3", server, keypair, contract);
    expect(res.status, `sesa3() call failed: ${toSafeJson(res)}`).to.equal("SUCCESS");
    expect(res.returnValue, `unexpected sesa3: ${toSafeJson(res)}`).to.equal(2n);
  });

  it('increment values', async () => {
    let incRes = await call_contract_function("inc", server, keypair, contract);
    expect(incRes.status, `inc() call failed: ${toSafeJson(incRes)}`).to.equal("SUCCESS");

    let res = await call_contract_function("sesa", server, keypair, contract);
    expect(res.returnValue).to.equal(2n);

    res = await call_contract_function("sesa1", server, keypair, contract);
    expect(res.returnValue).to.equal(2n);

    res = await call_contract_function("sesa2", server, keypair, contract);
    expect(res.returnValue).to.equal(3n);

    res = await call_contract_function("sesa3", server, keypair, contract);
    expect(res.returnValue).to.equal(3n);
  });

  it('decrement values', async () => {
    let decRes = await call_contract_function("dec", server, keypair, contract);
    expect(decRes.status, `dec() call failed: ${toSafeJson(decRes)}`).to.equal("SUCCESS");

    let res = await call_contract_function("sesa", server, keypair, contract);
    expect(res.returnValue).to.equal(1n);

    res = await call_contract_function("sesa1", server, keypair, contract);
    expect(res.returnValue).to.equal(1n);

    res = await call_contract_function("sesa2", server, keypair, contract);
    expect(res.returnValue).to.equal(2n);

    res = await call_contract_function("sesa3", server, keypair, contract);
    expect(res.returnValue).to.equal(2n);
  });
});
