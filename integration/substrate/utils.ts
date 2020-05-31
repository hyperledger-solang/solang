import { ApiPromise, SubmittableResult } from "@polkadot/api";
import { KeyringPair } from "@polkadot/keyring/types";
import { Option } from "@polkadot/types";
import { Address, ContractInfo, Hash, StorageData } from "@polkadot/types/interfaces";
import { u8aToHex } from "@polkadot/util";
import BN from "bn.js";
import fs from "fs";
import path from "path";
const blake = require('blakejs');

import { GAS_REQUIRED } from "./consts";

export async function sendAndReturnFinalized(signer: KeyringPair, tx: any) {
  return new Promise(function(resolve, reject) {
    tx.signAndSend(signer, (result: SubmittableResult) => {
      if (result.status.isInBlock) {
        // Return the result of the submittable extrinsic after the transfer is finalized
        resolve(result as SubmittableResult);
      }
      if (
        result.status.isDropped ||
        result.status.isInvalid ||
        result.status.isUsurped
      ) {
        reject(result as SubmittableResult);
        console.error("ERROR: Transaction could not be finalized.");
      }
    });
  });
}

export async function putCode(
  api: ApiPromise,
  signer: KeyringPair,
  fileName: string
): Promise<Hash> {
  const wasmCode = fs
    .readFileSync(path.join(__dirname, fileName))
    .toString("hex");
  const tx = api.tx.contracts.putCode(`0x${wasmCode}`);
  const result: any = await sendAndReturnFinalized(signer, tx);
  const record = result.findRecord("contracts", "CodeStored");

  if (!record) {
    console.error("ERROR: No code stored after executing putCode()");
  }
  // Return code hash.
  return record.event.data[0];
}

export async function instantiate(
  api: ApiPromise,
  signer: KeyringPair,
  codeHash: Hash,
  inputData: any,
  endowment: BN,
  gasRequired: number = GAS_REQUIRED
): Promise<Address> {
  const tx = api.tx.contracts.instantiate(
    endowment,
    gasRequired,
    codeHash,
    inputData
  );
  const result: any = await sendAndReturnFinalized(signer, tx);
  const record = result.findRecord("contracts", "Instantiated");

  if (!record) {
    console.error("ERROR: No new instantiated contract");
  }
  // Return the Address of  the instantiated contract.
  return record.event.data[1];
}

export async function callContract(
  api: ApiPromise,
  signer: KeyringPair,
  contractAddress: Address,
  inputData: any,
  gasRequired: number = GAS_REQUIRED,
  endowment: number = 0
): Promise<void> {
  const tx = api.tx.contracts.call(
    contractAddress,
    endowment,
    gasRequired,
    inputData
  );

  await sendAndReturnFinalized(signer, tx);
}

export async function getContractStorage(
  api: ApiPromise,
  contractAddress: Address,
  storageKey: Uint8Array
): Promise<StorageData> {
  const contractInfo = await api.query.contracts.contractInfoOf(
    contractAddress
  );
  // Return the value of the contracts storage
  const childStorageKey = (contractInfo as Option<ContractInfo>).unwrap().asAlive.trieId;
  // Add the default child_storage key prefix `:child_storage:default:` to the storage key
  const prefixedStorageKey = '0x3a6368696c645f73746f726167653a64656661756c743a' + u8aToHex(childStorageKey,-1,false);

  console.log(prefixedStorageKey)
  const storageKeyBlake2b = '0x' + blake.blake2bHex(storageKey, null, 32);

  const result =  await api.rpc.childstate.getStorage(
    prefixedStorageKey, // childStorageKey || prefixed trieId of the contract
    storageKeyBlake2b // hashed storageKey
  ) as Option<StorageData>;
  console.log(result.unwrapOrDefault())
  return result.unwrapOrDefault();
}
