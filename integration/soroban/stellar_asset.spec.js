import * as StellarSdk from '@stellar/stellar-sdk';
import { readFileSync, existsSync } from 'fs';
import { expect } from 'chai';
import path from 'path';
import { fileURLToPath } from 'url';
import { call_contract_function, toSafeJson } from './test_helpers.js';
import { Server } from '@stellar/stellar-sdk/rpc';

const __filename = fileURLToPath(import.meta.url);
const dirname = path.dirname(__filename);
const server = new Server("https://soroban-testnet.stellar.org");

// Helper function to create int128 ScVal from BigInt
function int128ToScVal(value) {
  // Convert BigInt to int128 ScVal
  // int128 is represented as two 64-bit parts (high and low)
  const MAX_UINT64 = 0xFFFFFFFFFFFFFFFFn;
  const low = value & MAX_UINT64;
  const high = value >> 64n;
  return StellarSdk.xdr.ScVal.scvI128(
    new StellarSdk.xdr.Int128Parts({
      hi: StellarSdk.xdr.Int64.fromString(high.toString()),
      lo: StellarSdk.xdr.Uint64.fromString(low.toString())
    })
  );
}

describe('Stellar Asset Contract', () => {
  let aliceKeypair, bobKeypair, contract;

  before(async function () {
    console.log('Setting up Stellar Asset Contract tests...');

    // Check if required files exist
    const alicePath = path.join(dirname, 'alice.txt');
    const bobPath = path.join(dirname, 'bob.txt');
    const contractIdPath = path.join(dirname, 'stellar_asset_contract_id.txt');

    if (!existsSync(alicePath)) {
      console.log('Skipping Stellar Asset Contract tests: alice.txt not found');
      this.skip();
      return;
    }

    if (!existsSync(bobPath)) {
      console.log('Skipping Stellar Asset Contract tests: bob.txt not found');
      this.skip();
      return;
    }

    if (!existsSync(contractIdPath)) {
      console.log('Skipping Stellar Asset Contract tests: stellar_asset_contract_id.txt not found');
      this.skip();
      return;
    }

    // Read secrets from files (with try-catch in case file is deleted between existsSync and readFileSync)
    let aliceSecret, bobSecret, contractId;
    try {
      aliceSecret = readFileSync(alicePath, 'utf8').trim();
      bobSecret = readFileSync(bobPath, 'utf8').trim();
      contractId = readFileSync(contractIdPath, 'utf8').trim();
    } catch (err) {
      console.log(`Skipping Stellar Asset Contract tests: error reading configuration files: ${err.message}`);
      this.skip();
      return;
    }

    if (!aliceSecret || !bobSecret || !contractId) {
      console.log('Skipping Stellar Asset Contract tests: missing required configuration');
      this.skip();
      return;
    }

    aliceKeypair = StellarSdk.Keypair.fromSecret(aliceSecret);
    bobKeypair = StellarSdk.Keypair.fromSecret(bobSecret);
    contract = new StellarSdk.Contract(contractId);
  });

  it('query initial balance for alice', async function () {
    if (!contract) {
      this.skip();
      return;
    }

    const aliceAddress = aliceKeypair.publicKey();
    const aliceAddressScVal = StellarSdk.Address.fromString(aliceAddress).toScVal();

    let res = await call_contract_function("balance", server, aliceKeypair, contract, aliceAddressScVal);
    expect(res.status, `Balance query failed: ${toSafeJson(res)}`).to.equal("SUCCESS");
    expect(typeof res.returnValue).to.equal('bigint');
    expect(res.returnValue >= 0n, `Balance should be non-negative: ${toSafeJson(res)}`).to.be.true;
  });

  it('mint tokens to alice', async function () {
    if (!contract) {
      this.skip();
      return;
    }

    const aliceAddress = aliceKeypair.publicKey();
    const aliceAddressScVal = StellarSdk.Address.fromString(aliceAddress).toScVal();
    const mintAmount = 1000n;
    const mintAmountScVal = int128ToScVal(mintAmount);

    // Get balance before mint
    let balanceBefore = await call_contract_function("balance", server, aliceKeypair, contract, aliceAddressScVal);
    expect(balanceBefore.status).to.equal("SUCCESS");
    const balanceBeforeValue = BigInt(balanceBefore.returnValue);

    // Mint tokens
    let mintRes = await call_contract_function("mint", server, aliceKeypair, contract, aliceAddressScVal, mintAmountScVal);
    expect(mintRes.status, `Mint failed: ${toSafeJson(mintRes)}`).to.equal("SUCCESS");

    // Verify balance increased
    let balanceAfter = await call_contract_function("balance", server, aliceKeypair, contract, aliceAddressScVal);
    expect(balanceAfter.status).to.equal("SUCCESS");
    expect(balanceAfter.returnValue, `Balance should increase by mint amount: ${toSafeJson(balanceAfter)}`).to.equal(balanceBeforeValue + mintAmount);
  });

  it('transfer tokens from alice to bob', async function () {
    if (!contract) {
      this.skip();
      return;
    }

    const aliceAddress = aliceKeypair.publicKey();
    const bobAddress = bobKeypair.publicKey();
    const aliceAddressScVal = StellarSdk.Address.fromString(aliceAddress).toScVal();
    const bobAddressScVal = StellarSdk.Address.fromString(bobAddress).toScVal();
    const transferAmount = 100n;
    const transferAmountScVal = int128ToScVal(transferAmount);

    // Get balances before transfer
    let aliceBalanceBefore = await call_contract_function("balance", server, aliceKeypair, contract, aliceAddressScVal);
    let bobBalanceBefore = await call_contract_function("balance", server, aliceKeypair, contract, bobAddressScVal);
    expect(aliceBalanceBefore.status).to.equal("SUCCESS");
    expect(bobBalanceBefore.status).to.equal("SUCCESS");
    const aliceBalanceBeforeValue = BigInt(aliceBalanceBefore.returnValue);
    const bobBalanceBeforeValue = BigInt(bobBalanceBefore.returnValue);

    // Transfer tokens
    let transferRes = await call_contract_function("transfer", server, aliceKeypair, contract, aliceAddressScVal, bobAddressScVal, transferAmountScVal);
    expect(transferRes.status, `Transfer failed: ${toSafeJson(transferRes)}`).to.equal("SUCCESS");

    // Verify balances after transfer
    let aliceBalanceAfter = await call_contract_function("balance", server, aliceKeypair, contract, aliceAddressScVal);
    let bobBalanceAfter = await call_contract_function("balance", server, aliceKeypair, contract, bobAddressScVal);
    expect(aliceBalanceAfter.status).to.equal("SUCCESS");
    expect(bobBalanceAfter.status).to.equal("SUCCESS");
    expect(aliceBalanceAfter.returnValue, `Alice balance should decrease: ${toSafeJson(aliceBalanceAfter)}`).to.equal(aliceBalanceBeforeValue - transferAmount);
    expect(bobBalanceAfter.returnValue, `Bob balance should increase: ${toSafeJson(bobBalanceAfter)}`).to.equal(bobBalanceBeforeValue + transferAmount);
  });

  it('approve tokens for transferFrom', async function () {
    if (!contract) {
      this.skip();
      return;
    }

    const aliceAddress = aliceKeypair.publicKey();
    const bobAddress = bobKeypair.publicKey();
    const aliceAddressScVal = StellarSdk.Address.fromString(aliceAddress).toScVal();
    const bobAddressScVal = StellarSdk.Address.fromString(bobAddress).toScVal();
    const approveAmount = 50n;
    const approveAmountScVal = int128ToScVal(approveAmount);

    // Approve bob to spend alice's tokens
    let approveRes = await call_contract_function("approve", server, aliceKeypair, contract, aliceAddressScVal, bobAddressScVal, approveAmountScVal);
    expect(approveRes.status, `Approve failed: ${toSafeJson(approveRes)}`).to.equal("SUCCESS");

    // Verify allowance
    let allowanceRes = await call_contract_function("allowance", server, aliceKeypair, contract, aliceAddressScVal, bobAddressScVal);
    expect(allowanceRes.status).to.equal("SUCCESS");
    expect(allowanceRes.returnValue, `Allowance should match approved amount: ${toSafeJson(allowanceRes)}`).to.equal(approveAmount);
  });

  it('transferFrom using approved allowance', async function () {
    if (!contract) {
      this.skip();
      return;
    }

    const aliceAddress = aliceKeypair.publicKey();
    const bobAddress = bobKeypair.publicKey();
    const aliceAddressScVal = StellarSdk.Address.fromString(aliceAddress).toScVal();
    const bobAddressScVal = StellarSdk.Address.fromString(bobAddress).toScVal();
    const transferAmount = 25n;
    const transferAmountScVal = int128ToScVal(transferAmount);

    // Get balances before transferFrom
    let aliceBalanceBefore = await call_contract_function("balance", server, aliceKeypair, contract, aliceAddressScVal);
    let bobBalanceBefore = await call_contract_function("balance", server, aliceKeypair, contract, bobAddressScVal);
    expect(aliceBalanceBefore.status).to.equal("SUCCESS");
    expect(bobBalanceBefore.status).to.equal("SUCCESS");
    const aliceBalanceBeforeValue = BigInt(aliceBalanceBefore.returnValue);
    const bobBalanceBeforeValue = BigInt(bobBalanceBefore.returnValue);

    // Transfer from alice to bob using bob's approval
    let transferFromRes = await call_contract_function("transfer_from", server, bobKeypair, contract, bobAddressScVal, aliceAddressScVal, bobAddressScVal, transferAmountScVal);
    expect(transferFromRes.status, `TransferFrom failed: ${toSafeJson(transferFromRes)}`).to.equal("SUCCESS");

    // Verify balances after transferFrom
    let aliceBalanceAfter = await call_contract_function("balance", server, aliceKeypair, contract, aliceAddressScVal);
    let bobBalanceAfter = await call_contract_function("balance", server, aliceKeypair, contract, bobAddressScVal);
    expect(aliceBalanceAfter.status).to.equal("SUCCESS");
    expect(bobBalanceAfter.status).to.equal("SUCCESS");
    expect(aliceBalanceAfter.returnValue, `Alice balance should decrease: ${toSafeJson(aliceBalanceAfter)}`).to.equal(aliceBalanceBeforeValue - transferAmount);
    expect(bobBalanceAfter.returnValue, `Bob balance should increase: ${toSafeJson(bobBalanceAfter)}`).to.equal(bobBalanceBeforeValue + transferAmount);

    // Verify allowance decreased
    let allowanceRes = await call_contract_function("allowance", server, aliceKeypair, contract, aliceAddressScVal, bobAddressScVal);
    expect(allowanceRes.status).to.equal("SUCCESS");
    expect(allowanceRes.returnValue, `Allowance should decrease: ${toSafeJson(allowanceRes)}`).to.equal(25n); // 50 - 25 = 25
  });

  it('burn tokens from alice', async function () {
    if (!contract) {
      this.skip();
      return;
    }

    const aliceAddress = aliceKeypair.publicKey();
    const aliceAddressScVal = StellarSdk.Address.fromString(aliceAddress).toScVal();
    const burnAmount = 10n;
    const burnAmountScVal = int128ToScVal(burnAmount);

    // Get balance before burn
    let balanceBefore = await call_contract_function("balance", server, aliceKeypair, contract, aliceAddressScVal);
    expect(balanceBefore.status).to.equal("SUCCESS");
    const balanceBeforeValue = BigInt(balanceBefore.returnValue);

    // Burn tokens
    let burnRes = await call_contract_function("burn", server, aliceKeypair, contract, aliceAddressScVal, burnAmountScVal);
    expect(burnRes.status, `Burn failed: ${toSafeJson(burnRes)}`).to.equal("SUCCESS");

    // Verify balance decreased
    let balanceAfter = await call_contract_function("balance", server, aliceKeypair, contract, aliceAddressScVal);
    expect(balanceAfter.status).to.equal("SUCCESS");
    expect(balanceAfter.returnValue, `Balance should decrease by burn amount: ${toSafeJson(balanceAfter)}`).to.equal(balanceBeforeValue - burnAmount);
  });

  it('transfer fails with insufficient balance', async function () {
    if (!contract) {
      this.skip();
      return;
    }

    const aliceAddress = aliceKeypair.publicKey();
    const bobAddress = bobKeypair.publicKey();
    const aliceAddressScVal = StellarSdk.Address.fromString(aliceAddress).toScVal();
    const bobAddressScVal = StellarSdk.Address.fromString(bobAddress).toScVal();

    // Get alice's current balance
    let balanceRes = await call_contract_function("balance", server, aliceKeypair, contract, aliceAddressScVal);
    expect(balanceRes.status).to.equal("SUCCESS");
    const currentBalance = BigInt(balanceRes.returnValue);

    // Try to transfer more than balance
    const excessiveAmount = currentBalance + 1000n;
    const excessiveAmountScVal = int128ToScVal(excessiveAmount);

    let transferRes = await call_contract_function("transfer", server, aliceKeypair, contract, aliceAddressScVal, bobAddressScVal, excessiveAmountScVal);
    expect(transferRes.status, `Transfer should fail with insufficient balance: ${toSafeJson(transferRes)}`).to.not.equal("SUCCESS");
    expect(transferRes.error, `Error should mention insufficient balance: ${toSafeJson(transferRes)}`).to.include("Insufficient balance");
  });
}); 

