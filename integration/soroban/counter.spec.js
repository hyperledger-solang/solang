import * as StellarSdk from '@stellar/stellar-sdk';
import { readFileSync } from 'fs';
import { expect } from 'chai';
import path from 'path';
import { fileURLToPath } from 'url';
import { call_contract_function } from './test_helpers.js';
import exp from 'constants';

const __filename = fileURLToPath(import.meta.url);
const dirname = path.dirname(__filename);

describe('Counter', () => {
  let keypair;
  const server = new StellarSdk.SorobanRpc.Server(
    "https://soroban-testnet.stellar.org:443",
  );
  console.log('server', server);
  let contractAddr;
  let contract;
  before(async () => {

    console.log('Setting up counter contract tests...');

    // read secret from file
    const secret = readFileSync('/Users/ahmadsameh/Desktop/Work/salahswork/Soroban/solang/integration/soroban/alice.txt', 'utf8').trim();
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
    console.log(`initial counter is: ${count}`);
    expect(count.toString()).eq("10");
  });

  it('increment counter', async () => {
    // increment the counter
    await call_contract_function("increment", server, keypair, contract);

    // get the count
    let count = await call_contract_function("count", server, keypair, contract);
    expect(count.toString()).eq("11");
  });

  it('adding two numbers', async () => {
    // add two numbers
    let args = [
      StellarSdk.xdr.ScVal.scvU32(30),
      StellarSdk.xdr.ScVal.scvU32(40)
    ];

    let result = await call_contract_function("additionu32", server, keypair, contract, ...args);    // let returnValue = result.returnValue().value().toString();
    console.log(`additionu32 output is: ${result}`);
    expect(result.toString()).eq("70");
  });
});


