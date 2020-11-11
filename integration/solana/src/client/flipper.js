// @flow

import {
  Account,
  Connection,
  BpfLoader,
  BPF_LOADER_PROGRAM_ID,
  PublicKey,
  LAMPORTS_PER_SOL,
  SystemProgram,
  TransactionInstruction,
  Transaction,
} from '@solana/web3.js';
import fs from 'mz/fs';
import * as BufferLayout from 'buffer-layout';

import { url, urlTls } from '../../url';
import { Store } from './util/store';
import { newAccountWithLamports } from './util/new-account-with-lamports';
import { sendAndConfirmTransaction } from './util/send-and-confirm-transaction';
import AbiCoder from 'web3-eth-abi';
import parse from 'binary';

/**
 * Connection to the network
 */
let connection: Connection;

/**
 * Connection to the network
 */
let payerAccount: Account;

/**
 * Account for contract storage
 */
let storageAccount: Account;
let returndataAccount: Account;

/**
 * Hello world's program id
 */
let programId: PublicKey;

/**
 * The public key of the account we are saying hello to
 */
let returndataPubkey: PublicKey;
let storagePubkey: PublicKey;

const pathToProgram = 'flipper.so';

/**
 * Layout of the storage account data
 */
const storageAccountDataLayout = BufferLayout.struct([
  BufferLayout.u32('numGreets'),
]);

/**
 * Establish a connection to the cluster
 */
export async function establishConnection(): Promise<void> {
  connection = new Connection(url, 'recent');
  const version = await connection.getVersion();
  console.log('Connection to cluster established:', url, version);
}

/**
 * Establish an account to pay for everything
 */
export async function establishPayer(): Promise<void> {
  if (!payerAccount) {
    let fees = 0;
    const { feeCalculator } = await connection.getRecentBlockhash();

    // Calculate the cost to load the program
    const data = await fs.readFile(pathToProgram);
    const NUM_RETRIES = 500; // allow some number of retries
    fees +=
      feeCalculator.lamportsPerSignature *
      (BpfLoader.getMinNumSignatures(data.length) + NUM_RETRIES) +
      (await connection.getMinimumBalanceForRentExemption(data.length));

    // Calculate the cost to fund the greeter account
    fees += await await connection.getMinimumBalanceForRentExemption(
      storageAccountDataLayout.span,
    );

    // Calculate the cost of sending the transactions
    fees += feeCalculator.lamportsPerSignature * 100; // wag

    // Fund a new payer via airdrop
    payerAccount = await newAccountWithLamports(connection, fees);
  }

  const lamports = await connection.getBalance(payerAccount.publicKey);
  console.log(
    'Using account',
    payerAccount.publicKey.toBase58(),
    'containing',
    lamports / LAMPORTS_PER_SOL,
    'Sol to pay for fees',
  );
}

/**
 * Load the hello world BPF program if not already loaded
 */
export async function loadProgram(): Promise<void> {
  const store = new Store();

  // Check if the program has already been loaded
  try {
    let config = await store.load('config.json');
    programId = new PublicKey(config.programId);
    storagePubkey = new PublicKey(config.storagePubkey);
    returndataPubkey = new PublicKey(config.returndataPubkey);
    await connection.getAccountInfo(programId);
    console.log('Program already loaded to account', programId.toBase58());
    return;
  } catch (err) {
    // try to load the program
  }

  // Load the program
  console.log('Loading flipper program...');
  const data = await fs.readFile(pathToProgram);
  const programAccount = new Account();
  await BpfLoader.load(
    connection,
    payerAccount,
    programAccount,
    data,
    BPF_LOADER_PROGRAM_ID,
  );
  programId = programAccount.publicKey;
  console.log('Program loaded to account', programId.toBase58());

  // Create the return data account
  returndataAccount = new Account();
  returndataPubkey = returndataAccount.publicKey;
  {
    console.log('Creating account', returndataPubkey.toBase58(), 'for flipper return data');
    const returndataSpace = 100;
    const returndataLamports = await connection.getMinimumBalanceForRentExemption(
      returndataSpace
    );
    const transaction = new Transaction().add(
      SystemProgram.createAccount({
        fromPubkey: payerAccount.publicKey,
        newAccountPubkey: returndataPubkey,
        lamports: returndataLamports,
        space: returndataSpace,
        programId,
      }),
    );
    await sendAndConfirmTransaction(
      'createAccount',
      connection,
      transaction,
      payerAccount,
      returndataAccount,
    );
  }

  // Create the contract storage account
  storageAccount = new Account();
  storagePubkey = storageAccount.publicKey;
  {
    console.log('Creating account', storagePubkey.toBase58(), 'for flipper contract storage');
    const space = 8;
    const lamports = await connection.getMinimumBalanceForRentExemption(
      storageAccountDataLayout.span,
    );
    const transaction = new Transaction().add(
      SystemProgram.createAccount({
        fromPubkey: payerAccount.publicKey,
        newAccountPubkey: storagePubkey,
        lamports,
        space,
        programId,
      }),
    );

    await sendAndConfirmTransaction(
      'createAccount',
      connection,
      transaction,
      payerAccount,
      storageAccount,
    );
  }

  // Save this info for next time
  await store.save('config.json', {
    url: urlTls,
    programId: programId.toBase58(),
    storagePubkey: storagePubkey.toBase58(),
    returndataPubkey: returndataPubkey.toBase58(),
  });
}

/**
 * Call constructor
 */
export async function callConstructor(): Promise<void> {
  console.log('Calling constructor', storagePubkey.toBase58());

  const constructor_input = AbiCoder.encodeParameters(['bool'], [true]);

  // A solidity contract should always have its constructor called, even if there is not
  // one defined in the source code. This handles setting up storage initializers

  // A solidity contract always needs an account for its return data. Any call, including
  // a constructor, can revert with an error string. This revert string will be placed into
  // the return data account.

  // The second account is for storing the contract data.
  const instruction = new TransactionInstruction({
    keys: [
      { pubkey: returndataPubkey, isSigner: false, isWritable: true },
      { pubkey: storagePubkey, isSigner: false, isWritable: true }],
    programId,
    data: Buffer.from(constructor_input.substring(2), 'hex'),
  });

  await sendAndConfirmTransaction(
    'callConstructor',
    connection,
    new Transaction().add(instruction),
    payerAccount,
  );
}

/**
 * Call get function
 */
export async function callGet(): Promise<boolean> {
  console.log('Calling function get', storagePubkey.toBase58());

  const flipper_abi = JSON.parse(await fs.readFile('flipper.abi'));
  const get_input = AbiCoder.encodeFunctionCall(flipper_abi.find(e => e.name == 'get'), []);

  // First account is for return buffer
  // Second account is for contract storage
  const instruction = new TransactionInstruction({
    keys: [
      { pubkey: returndataPubkey, isSigner: false, isWritable: true },
      { pubkey: storagePubkey, isSigner: false, isWritable: true }],
    programId,
    data: Buffer.from(get_input.substr(2), 'hex'),
  });
  await sendAndConfirmTransaction(
    'callGet',
    connection,
    new Transaction().add(instruction),
    payerAccount
  );

  // Unpack our return data. First retrieve the data for the account
  const accountInfo = await connection.getAccountInfo(returndataPubkey);

  // The first 8 bytes is the length of the data, followed by the data itself
  let result = parse(accountInfo.data).word64lu('length').buffer('data', 'length').vars;

  return AbiCoder.decodeParameters(['bool'], result.data.toString('hex'))[0];
}

/**
 * Call flip function
 */
export async function callFlip(): Promise<void> {
  console.log('Calling function flip', storagePubkey.toBase58());

  const flipper_abi = JSON.parse(await fs.readFile('flipper.abi'));
  const flip_input = AbiCoder.encodeFunctionCall(flipper_abi.find(e => e.name == 'flip'), []);

  const instruction = new TransactionInstruction({
    keys: [
      { pubkey: returndataPubkey, isSigner: false, isWritable: true },
      { pubkey: storagePubkey, isSigner: false, isWritable: true }],
    programId,
    data: Buffer.from(flip_input.substr(2), 'hex'),
  });
  await sendAndConfirmTransaction(
    'callFlip',
    connection,
    new Transaction().add(instruction),
    payerAccount
  );
}
