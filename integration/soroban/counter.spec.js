import * as StellarSdk from '@stellar/stellar-sdk';
import { readFileSync } from 'fs';
import { expect } from 'chai';
import path from 'path';
import { fileURLToPath } from 'url';
import { call_contract_function } from './test_helpers.js';

const __filename = fileURLToPath(import.meta.url);
const dirname = path.dirname(__filename);

describe('Counter', () => {
  let keypair;
  const server = new StellarSdk.SorobanRpc.Server(
    "https://soroban-testnet.stellar.org:443",
  );

  let contractAddr;
  let contract;
  before(async () => {

    console.log('Setting up counter contract tests...');

    // read secret from file
    const secret = readFileSync('alice.txt', 'utf8').trim();
    keypair = StellarSdk.Keypair.fromSecret(secret);

    let contractIdFile = path.join(dirname, '.soroban', 'contract-ids', 'counter.txt');
    // read contract address from file
    contractAddr = readFileSync(contractIdFile, 'utf8').trim().toString();

    // load contract
    contract = new StellarSdk.Contract(contractAddr);
  });

  it('get correct initial counter', async () => {
    // get the count
    let count = await call_contract_function("count", server, keypair, contract);
    console.log(count.returnValue().value());
    expect(count.returnValue().value().toString()).eq("10");
  });

  it('increment counter', async () => {
    // increment the counter
    await call_contract_function("increment", server, keypair, contract);

    // get the count
    let count = await call_contract_function("count", server, keypair, contract);
    expect(count.returnValue().value().toString()).eq("11");
  });

  it('adding two u64 values', async () => {
    // add two numbers

    let args = [
      StellarSdk.xdr.ScVal.scvU64(100n),
      StellarSdk.xdr.ScVal.scvU64(200n)
    ];

    console.log(`addingu64 inputs are: ${args[0].u64()} and ${args[1].u64()} `);


    let result = await call_contract_function("addingu64", server, keypair, contract, ...args);    // let returnValue = result.returnValue().value().toString();

    let output = result.returnValue().value().toString();
    console.log(`additionu64 output is: ${output}`);
    expect(result.returnValue().value().toString()).eq("300");
  });





  it('adding two u32 values', async () => {
    // add two numbers

    let args = [
      StellarSdk.xdr.ScVal.scvU32(50),
      StellarSdk.xdr.ScVal.scvU32(60)
    ];

    console.log(`additionu32 input is: ${args[0].u32()} and ${args[1].u32()} `);


    let result = await call_contract_function("addingu32", server, keypair, contract, ...args);    // let returnValue = result.returnValue().value().toString();

    let output = result.returnValue().value().toString();
    console.log(`additionu32 output is: ${output}`);
    expect(result.returnValue().value().toString()).eq("110");
  });
});


