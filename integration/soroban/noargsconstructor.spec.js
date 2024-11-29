import * as StellarSdk from '@stellar/stellar-sdk';
import { readFileSync } from 'fs';
import { expect } from 'chai';
import path from 'path';
import { fileURLToPath } from 'url';
import { call_contract_function } from './test_helpers.js';

const __filename = fileURLToPath(import.meta.url);
const dirname = path.dirname(__filename);

describe('CounterWithNoArgsConstructor', () => {
  let keypair;
  const server = new StellarSdk.SorobanRpc.Server(
    "https://soroban-testnet.stellar.org:443",
  );

  let contractAddr;
  let contract;
  before(async () => {

    console.log('Setting up counter with no-args constructor contract tests...');

    // read secret from file
    const secret = readFileSync('alice.txt', 'utf8').trim();
    keypair = StellarSdk.Keypair.fromSecret(secret);

    let contractIdFile = path.join(dirname, '.soroban', 'contract-ids', 'noargsconstructor.txt');
    // read contract address from file
    contractAddr = readFileSync(contractIdFile, 'utf8').trim().toString();

    // load contract
    contract = new StellarSdk.Contract(contractAddr);
  });

  it('make sure the constructor of the contract was called', async () => {
    // get the count
    let count = await call_contract_function("get", server, keypair, contract);
    expect(count.toString()).eq("2");
  });

});
