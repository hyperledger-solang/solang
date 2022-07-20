import fs, { PathLike } from 'fs';
const { ApiPromise, WsProvider, Keyring } = require('@polkadot/api');
import { CodePromise } from '@polkadot/api-contract';
import type { CodecArg } from '@polkadot/types/types';
import { SubmittableExtrinsic } from '@polkadot/api/types';
import { ISubmittableResult } from '@polkadot/types/types';
import { KeyringPair } from '@polkadot/keyring/types';

const default_url: string = "ws://localhost:9944";
export const gasLimit: bigint = 200000n * 1000000n;

export function aliceKeypair(): KeyringPair {
  const keyring = new Keyring({ type: 'sr25519' });
  return keyring.addFromUri('//Alice');
}

export function daveKeypair(): KeyringPair {
  const keyring = new Keyring({ type: 'sr25519' });
  return keyring.addFromUri('//Dave');
}

export async function createConnection(): Promise<ApiPromise> {
  let url = process.env.RPC_URL || default_url;

  return ApiPromise.create({ provider: new WsProvider(url) });
}

export async function deploy(api: ApiPromise, pair: KeyringPair, file: PathLike, ...params: CodecArg[]): Promise<any> {
  const contractJson = fs.readFileSync(file, { encoding: 'utf-8' });

  const code = new CodePromise(api, contractJson, null);

  const tx = code.tx.new({ gasLimit, value: BigInt(1e18) }, ...params);

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

export async function transaction(tx: SubmittableExtrinsic<"promise", ISubmittableResult>, pair: KeyringPair): Promise<ISubmittableResult> {
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
