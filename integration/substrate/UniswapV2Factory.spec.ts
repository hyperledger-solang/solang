import expect from 'expect';
import { weight, createConnection, deploy, transaction, aliceKeypair, daveKeypair, query } from './index';
import { ContractPromise } from '@polkadot/api-contract';
import { ApiPromise } from '@polkadot/api';
import { KeyringPair } from '@polkadot/keyring/types';
import { DecodedEvent } from '@polkadot/api-contract/types';

const TEST_ADDRESSES: [string, string] = [
  '5C4hrfjw9DjXZTzV3MwzrrAr9P1MJhSrvWGWqi1eSuyUv7BA',
  '5C4hrfjw9DjXZTzV3MwzrrAr9P1MJhSrvWGWqi1eSuyV1W6M'
]

describe('UniswapV2Factory', () => {
  let conn: ApiPromise;
  let factory: ContractPromise;
  let alice: KeyringPair;
  let dave: KeyringPair;
  let pairAbi: any;

  beforeEach(async function () {
    conn = await createConnection();

    alice = aliceKeypair();
    dave = daveKeypair();

    let deploy_contract = await deploy(conn, alice, 'UniswapV2Factory.contract', 10000000000000000n, alice.address);

    factory = new ContractPromise(conn, deploy_contract.abi, deploy_contract.address);

    // Upload UniswapV2Pair contract code so that it can instantiated from the factory
    // there probably is a better way of doing this than deploying a contract. Patches welcome.
    let pair = await deploy(conn, alice, 'UniswapV2Pair.contract', 0n);

    pairAbi = pair.abi;
  });

  afterEach(async function () {
    await conn.disconnect();
  });

  it('feeTo, feeToSetter, allPairsLength', async () => {
    const { output: feeTo } = await query(conn, alice, factory, "feeTo");
    // This is the 32-byte 0-address in ss58 format
    expect(feeTo?.eq('5C4hrfjw9DjXZTzV3MwzrrAr9P1MJhSrvWGWqi1eSuyUpnhM')).toBeTruthy();

    const { output: feeToSetter } = await query(conn, alice, factory, "feeToSetter");
    expect(feeToSetter?.eq(alice.address)).toBeTruthy();

    const { output: allPairsLength } = await query(conn, alice, factory, "allPairsLength");
    expect(allPairsLength?.eq(0)).toBeTruthy();
  })

  async function createPair(tokens: [string, string]) {
    let gasLimit = await weight(conn, factory, "createPair", [tokens[0], tokens[1]]);
    let tx = factory.tx.createPair({ gasLimit }, ...tokens);

    let res0: any = await transaction(tx, alice);

    let events: DecodedEvent[] = res0.contractEvents;
    expect(events.length).toEqual(1)
    expect(events[0].event.identifier).toBe('PairCreated')
    expect(events[0].args[0].toString()).toBe(TEST_ADDRESSES[0])
    expect(events[0].args[1].toString()).toBe(TEST_ADDRESSES[1])
    expect(events[0].args[3].eq(1)).toBeTruthy();

    let pair_address = events[0].args[2].toString();

    const { output: get_pair } = await query(conn, alice, factory, "getPair", [tokens[0], tokens[1]]);
    expect(get_pair?.eq(pair_address)).toBeTruthy();

    const { output: pairRev } = await query(conn, alice, factory, "getPair", [tokens[1], tokens[0]]);
    expect(pairRev?.eq(pair_address)).toBeTruthy();

    const { output: pair0 } = await query(conn, alice, factory, "allPairs", [0]);
    expect(pair0?.eq(pair_address)).toBeTruthy();

    const { output: pairLength } = await query(conn, alice, factory, "allPairsLength");
    expect(pairLength?.eq(1)).toBeTruthy();

    const pair = new ContractPromise(conn, pairAbi, pair_address);

    const { output: pair_factory } = await query(conn, alice, pair, "factory");
    expect(pair_factory?.eq(factory.address)).toBeTruthy();

    const { output: token0 } = await query(conn, alice, pair, "token0");
    expect(token0?.eq(TEST_ADDRESSES[0])).toBeTruthy();

    const { output: token1 } = await query(conn, alice, pair, "token1");
    expect(token1?.eq(TEST_ADDRESSES[1])).toBeTruthy();
  }

  it('createPair', async () => {
    await createPair(TEST_ADDRESSES)
  })

  it('createPair:reverse', async () => {
    await createPair(TEST_ADDRESSES.slice().reverse() as [string, string])
  })

  it('setFeeTo', async () => {
    let gasLimit = await weight(conn, factory, "setFeeTo", [dave.address]);
    let tx = factory.tx.setFeeTo({ gasLimit }, dave.address);
    await transaction(tx, alice);

    const { output: feeTo } = await query(conn, alice, factory, "feeTo");
    expect(feeTo?.eq(dave.address)).toBeTruthy();
  })

  it('setFeeToSetter', async () => {
    let gasLimit = await weight(conn, factory, "setFeeToSetter", [dave.address]);
    let tx = factory.tx.setFeeToSetter({ gasLimit }, dave.address);
    await transaction(tx, alice);

    const { output: feeTo } = await query(conn, alice, factory, "feeToSetter");
    expect(feeTo?.eq(dave.address)).toBeTruthy();
  })
})
