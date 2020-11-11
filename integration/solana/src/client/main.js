/**
 * Flipper
 *
 * @flow
 */

import {
  establishConnection,
  establishPayer,
  loadProgram,
  callConstructor,
  callFlip,
  callGet,
} from './flipper';

async function main() {
  console.log("Let's try out lfipper to a Solana account...");

  // Establish connection to the cluster
  await establishConnection();

  // Determine who pays for the fees
  await establishPayer();

  // Load the program if not already loaded
  await loadProgram();

  await callConstructor();

  let ret = await callGet();
  if (ret !== true) {
    throw new Error('flip value should be true after constructor');
  }
  await callFlip();
  ret = await callGet();
  if (ret !== false) {
    throw new Error('flip value should be false after 1st flip');
  }
  await callFlip();
  ret = await callGet();
  if (ret !== true) {
    throw new Error('flip value should be true after 2nd flip');
  }
  console.log('Success');
}

main()
  .catch(err => {
    console.error(err);
    process.exit(1);
  })
  .then(() => process.exit());
