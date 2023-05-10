import '@polkadot/api-augment';
import fs, { PathLike } from 'fs';
import { ApiPromise, WsProvider, Keyring } from '@polkadot/api';
import { convertWeight } from '@polkadot/api-contract/base/util';
import { CodePromise, ContractPromise } from '@polkadot/api-contract';
import { SubmittableExtrinsic } from '@polkadot/api/types';
import { Codec, ISubmittableResult } from '@polkadot/types/types';
import { KeyringPair } from '@polkadot/keyring/types';
import expect from 'expect';
import { ContractExecResultResult, WeightV2 } from '@polkadot/types/interfaces';

const default_url = "ws://127.0.0.1:9944";

export function aliceKeypair(): KeyringPair {
  const keyring = new Keyring({ type: 'sr25519' });
  return keyring.addFromUri('//Alice');
}

export function daveKeypair(): KeyringPair {
  const keyring = new Keyring({ type: 'sr25519' });
  return keyring.addFromUri('//Dave');
}

export function createConnection(): Promise<ApiPromise> {
  const url = process.env.RPC_URL || default_url;

  return ApiPromise.create({ provider: new WsProvider(url) });
}

export function deploy(api: ApiPromise, pair: KeyringPair, file: PathLike, value: bigint, ...params: unknown[]): Promise<any> {
  const contractJson = fs.readFileSync(file, { encoding: 'utf-8' });

  const code = new CodePromise(api, contractJson, null);

  const gasLimit = api.registry.createType('WeightV2', { refTime: 100000n * 1000000n, proofSize: 100000n });
  const tx = code.tx.new({ gasLimit, value }, ...params);

  return new Promise(async (resolve, reject) => {
    const unsub = await tx.signAndSend(pair, (result: any) => {
      if (result.status.isInBlock || result.status.isFinalized) {
        resolve(result.contract);
        unsub();
      }

      if (result.isError) {
        if (result.dispatchError) {
          console.log(result.dispatchError.toHuman());
        } else {
          console.log(result.asError.toHuman());
        }

        reject(result);
        unsub();
      }
    });
  });
}

export function transaction(tx: SubmittableExtrinsic<"promise", ISubmittableResult>, pair: KeyringPair): Promise<ISubmittableResult> {
  return new Promise(async (resolve, reject) => {
    const unsub = await tx.signAndSend(pair, (result: any) => {
      if (result.dispatchError) {
        console.log(`dispatchError:${JSON.stringify(result)}`)
        reject(result);
        unsub();
      }

      if (result.isError) {
        console.log(`isError:${JSON.stringify(result)}`)
        reject(result);
        unsub();
      }

      if (result.status.isInBlock || result.status.isFinalized) {
        resolve(result);
        unsub();
      }
    });
  });
}

// Returns the required gas estimated from a dry run
export async function weight(api: ApiPromise, contract: ContractPromise, message: string, args?: unknown[], value?: number) {
  let res = await dry_run(api, contract, message, args, value);
  return res.gasRequired
}

// Returns the debug buffer from the dry run result
export async function debug_buffer(api: ApiPromise, contract: ContractPromise, message: string, args?: unknown[], value?: number) {
  let res = await dry_run(api, contract, message, args, value);
  return res.debugMessage.toHuman()
}

// Return dry run result
export async function dry_run(api: ApiPromise, contract: ContractPromise, message: string, args?: unknown[], value?: number) {
  const ALICE = new Keyring({ type: 'sr25519' }).addFromUri('//Alice').address;
  const msg = contract.abi.findMessage(message);
  const dry = await api.call.contractsApi.call(ALICE, contract.address, value ? value : 0, null, null, msg.toU8a(args ? args : []));
  return dry;
}

// FIXME: The old contract.query API does not support WeightV2 yet
export async function query(
  api: ApiPromise,
  account: KeyringPair,
  contract: ContractPromise,
  message: string,
  args?: unknown[],
  value?: number,
  gasLimit?: WeightV2 | { refTime?: any; proofSize?: any; }
): Promise<{ output: Codec | null, result: ContractExecResultResult }> {
  const msg = contract.abi.findMessage(message);
  const callResult = await api.call.contractsApi.call(account.address, contract.address, value ? value : 0, gasLimit ? gasLimit : null, null, msg.toU8a(args ? args : []));
  // Same logic as contracts UI, so should be fine.
  // Refernce implementation: https://github.com/paritytech/contracts-ui/blob/e343221a0d5c1ae67122fe99028246e5bdf38c46/src/ui/hooks/useDecodedOutput.ts
  const output = callResult.result.isOk && msg.returnType
    ? contract.abi.registry.createTypeUnsafe(
      msg.returnType.lookupName || msg.returnType.type,
      [callResult.result.asOk.data.toU8a(true)],
      { isPedantic: true }
    )
    : null;
  expect(output !== null && typeof output === 'object' && 'Err' in output).toBeFalsy();
  return { output, result: callResult.result };
}
