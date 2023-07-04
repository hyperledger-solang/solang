import expect from 'expect';
import { weight, query, createConnection, deploy, transaction, aliceKeypair, daveKeypair } from './index';
import { ContractPromise } from '@polkadot/api-contract';
import { ApiPromise } from '@polkadot/api';
import { KeyringPair } from '@polkadot/keyring/types';

const MINIMUM_LIQUIDITY = BigInt(1000);
const TOTAL_SUPPLY = BigInt(10000e18);

describe('UniswapV2Pair', () => {
    let conn: ApiPromise;
    let factory: ContractPromise;
    let pair: ContractPromise;
    let token0: ContractPromise;
    let token1: ContractPromise;
    let alice: KeyringPair;
    let dave: KeyringPair;

    beforeEach(async function () {
        conn = await createConnection();

        alice = aliceKeypair();
        dave = daveKeypair();

        // Upload UniswapV2Pair contract code so that it can instantiated from the factory
        // there probably is a better way of doing this than deploying a contract. Patches welcome.
        const pairTmp = await deploy(conn, alice, 'UniswapV2Pair.contract', 0n);

        const pairAbi = pairTmp.abi;

        let deploy_contract = await deploy(conn, alice, 'UniswapV2Factory.contract', 10000000000000000n, alice.address);

        factory = new ContractPromise(conn, deploy_contract.abi, deploy_contract.address);

        const tokenA_contract = await deploy(conn, alice, 'ERC20.contract', 0n, TOTAL_SUPPLY);

        const tokenA = new ContractPromise(conn, tokenA_contract.abi, tokenA_contract.address);

        const tokenB_contract = await deploy(conn, alice, 'ERC20.contract', 0n, TOTAL_SUPPLY);

        const tokenB = new ContractPromise(conn, tokenB_contract.abi, tokenB_contract.address);

        let gasLimit = await weight(conn, factory, "createPair", [tokenA.address, tokenB.address]);
        let tx = factory.tx.createPair({ gasLimit }, tokenA.address, tokenB.address);

        await transaction(tx, alice);

        const { output: get_pair } = await query(conn, alice, factory, "getPair", [tokenA.address, tokenB.address]);

        pair = new ContractPromise(conn, pairAbi, get_pair!.toString());

        const { output: token0_address } = await query(conn, alice, pair, "token0");

        if (tokenA.address.toString() == token0_address!.toString()) {
            token0 = tokenA;
            token1 = tokenB;
        } else {
            expect(tokenB.address.toString()).toEqual(token0_address!.toString());
            token0 = tokenB;
            token1 = tokenA;
        }
    });

    afterEach(async function () {
        await conn.disconnect();
    });

    it('mint', async () => {
        const token0Amount = BigInt(1e18)
        const token1Amount = BigInt(4e18)

        let gasLimit = await weight(conn, token0, "transfer", [pair.address, token0Amount]);
        let tx = token0.tx.transfer({ gasLimit }, pair.address, token0Amount);
        await transaction(tx, alice);
        gasLimit = await weight(conn, token1, "transfer", [pair.address, token1Amount]);
        tx = token1.tx.transfer({ gasLimit }, pair.address, token1Amount);
        await transaction(tx, alice);
        const expectedLiquidity = BigInt(2e18)

        gasLimit = await weight(conn, pair, "mint", [alice.address]);
        tx = pair.tx.mint({ gasLimit }, alice.address);
        await transaction(tx, alice);

        const { output: totalSupply } = await query(conn, alice, pair, "totalSupply");
        expect(totalSupply?.eq(expectedLiquidity)).toBeTruthy();
        const { output: bal } = await query(conn, alice, pair, "balanceOfAddress", [alice.address]);
        expect(bal?.eq(expectedLiquidity - MINIMUM_LIQUIDITY)).toBeTruthy();
        const { output: bal0 } = await query(conn, alice, token0, "balanceOf", [pair.address]);
        expect(bal0?.eq(token0Amount)).toBeTruthy();
        const { output: bal1 } = await query(conn, alice, token1, "balanceOf", [pair.address]);
        expect(bal1?.eq(token1Amount)).toBeTruthy();
        const { output: reserves } = await query(conn, alice, pair, "getReserves");
        // surely there must be a better way.
        let v: any = reserves;
        expect(v[0].eq(token0Amount)).toBeTruthy();
        expect(v[1].eq(token1Amount)).toBeTruthy();
    })

    async function addLiquidity(token0Amount: BigInt, token1Amount: BigInt) {
        let gasLimit = await weight(conn, token0, "transfer", [pair.address, token0Amount]);
        let tx = token0.tx.transfer({ gasLimit }, pair.address, token0Amount);
        await transaction(tx, alice);
        gasLimit = await weight(conn, token1, "transfer", [pair.address, token1Amount]);
        tx = token1.tx.transfer({ gasLimit }, pair.address, token1Amount);
        await transaction(tx, alice);
        gasLimit = await weight(conn, pair, "mint", [alice.address]);
        tx = pair.tx.mint({ gasLimit }, alice.address);
        await transaction(tx, alice);
    }

    it('swap:token0', async () => {
        const token0Amount = BigInt(5e18)
        const token1Amount = BigInt(10e18)
        await addLiquidity(token0Amount, token1Amount)

        const swapAmount = BigInt(1e18)
        const expectedOutputAmount = BigInt(1662497915624478906)

        let gasLimit = await weight(conn, token0, "transfer", [pair.address, swapAmount]);
        let tx = token0.tx.transfer({ gasLimit }, pair.address, swapAmount);
        await transaction(tx, alice);

        gasLimit = await weight(conn, pair, "swap", [0, expectedOutputAmount, alice.address, '']);
        tx = pair.tx.swap({ gasLimit }, 0, expectedOutputAmount, alice.address, '');
        await transaction(tx, alice);

        const { output: reserves } = await query(conn, alice, pair, "getReserves");
        // surely there must be a better way.
        let v: any = reserves;
        expect(v[0].eq(token0Amount + swapAmount)).toBeTruthy();
        expect(v[1].eq(token1Amount - expectedOutputAmount)).toBeTruthy();

        const { output: bal0 } = await query(conn, alice, token0, "balanceOf", [pair.address]);
        expect(bal0?.eq(token0Amount + swapAmount)).toBeTruthy();
        const { output: bal1 } = await query(conn, alice, token1, "balanceOf", [pair.address]);
        expect(bal1?.eq(token1Amount - expectedOutputAmount)).toBeTruthy();

        const { output: returnTotalSupplyToken0 } = await query(conn, alice, token0, "totalSupply");
        const { output: returnTotalSupplyToken1 } = await query(conn, alice, token1, "totalSupply");
        const { output: walletBal0 } = await query(conn, alice, token0, "balanceOf", [alice.address]);
        const { output: walletBal1 } = await query(conn, alice, token1, "balanceOf", [alice.address]);

        const totalSupplyToken0 = BigInt(returnTotalSupplyToken0!.toString());
        const totalSupplyToken1 = BigInt(returnTotalSupplyToken1!.toString());

        expect(walletBal0?.eq(totalSupplyToken0 - token0Amount - swapAmount)).toBeTruthy();
        expect(walletBal1?.eq(totalSupplyToken1 - token1Amount + expectedOutputAmount)).toBeTruthy();
    })

    it('swap:token1', async () => {
        const token0Amount = BigInt(5e18)
        const token1Amount = BigInt(10e18)
        await addLiquidity(token0Amount, token1Amount)

        const swapAmount = BigInt(1e18)
        const expectedOutputAmount = BigInt(453305446940074565)

        let gasLimit = await weight(conn, token1, "transfer", [pair.address, swapAmount]);
        let tx = token1.tx.transfer({ gasLimit }, pair.address, swapAmount);
        await transaction(tx, alice);

        gasLimit = await weight(conn, pair, "swap", [expectedOutputAmount, 0, alice.address, '']);
        tx = pair.tx.swap({ gasLimit }, expectedOutputAmount, 0, alice.address, '');
        await transaction(tx, alice);

        const { output: reserves } = await query(conn, alice, pair, "getReserves");
        // surely there must be a better way.
        let v: any = reserves;
        expect(v[0].eq(token0Amount - expectedOutputAmount)).toBeTruthy();
        expect(v[1].eq(token1Amount + swapAmount)).toBeTruthy();

        const { output: bal0 } = await query(conn, alice, token0, "balanceOf", [pair.address]);
        expect(bal0?.eq(token0Amount - expectedOutputAmount)).toBeTruthy();
        const { output: bal1 } = await query(conn, alice, token1, "balanceOf", [pair.address]);
        expect(bal1?.eq(token1Amount + swapAmount)).toBeTruthy();

        const { output: returnTotalSupplyToken0 } = await query(conn, alice, token0, "totalSupply");
        const { output: returnTotalSupplyToken1 } = await query(conn, alice, token1, "totalSupply");
        const { output: walletBal0 } = await query(conn, alice, token0, "balanceOf", [alice.address]);
        const { output: walletBal1 } = await query(conn, alice, token1, "balanceOf", [alice.address]);

        const totalSupplyToken0 = BigInt(returnTotalSupplyToken0!.toString());
        const totalSupplyToken1 = BigInt(returnTotalSupplyToken1!.toString());

        expect(walletBal0?.eq(totalSupplyToken0 - token0Amount + expectedOutputAmount)).toBeTruthy();
        expect(walletBal1?.eq(totalSupplyToken1 - token1Amount - swapAmount)).toBeTruthy();
    })

    it('burn', async () => {
        const token0Amount = BigInt(3e18)
        const token1Amount = BigInt(3e18)
        await addLiquidity(token0Amount, token1Amount)

        const expectedLiquidity = BigInt(3e18)

        let gasLimit = await weight(conn, pair, "transferAddressUint256", [pair.address, expectedLiquidity - MINIMUM_LIQUIDITY]);
        let tx = pair.tx.transferAddressUint256({ gasLimit }, pair.address, expectedLiquidity - MINIMUM_LIQUIDITY);
        await transaction(tx, alice);

        gasLimit = await weight(conn, pair, "burn", [alice.address]);
        tx = pair.tx.burn({ gasLimit }, alice.address);
        await transaction(tx, alice);

        const { output: walletBal0 } = await query(conn, alice, pair, "balanceOfAddress", [alice.address]);
        expect(walletBal0?.eq(0)).toBeTruthy();

        const { output: pairTotalSupply } = await query(conn, alice, pair, "totalSupply");
        expect(pairTotalSupply?.eq(MINIMUM_LIQUIDITY)).toBeTruthy();

        const { output: token0pairBal } = await query(conn, alice, token0, "balanceOf", [pair.address]);
        expect(token0pairBal?.eq(1000)).toBeTruthy();
        const { output: token1pairBal } = await query(conn, alice, token1, "balanceOf", [pair.address]);
        expect(token1pairBal?.eq(1000)).toBeTruthy();

        const { output: retToken0TotalSupply } = await query(conn, alice, token0, "totalSupply");
        const { output: retToken1TotalSupply } = await query(conn, alice, token1, "totalSupply");

        const totalSupplyToken0 = BigInt(retToken0TotalSupply!.toString());
        const totalSupplyToken1 = BigInt(retToken1TotalSupply!.toString());

        const { output: bal0 } = await query(conn, alice, token0, "balanceOf", [alice.address]);
        expect(bal0?.eq(totalSupplyToken0 - 1000n)).toBeTruthy();
        const { output: bal1 } = await query(conn, alice, token1, "balanceOf", [alice.address]);
        expect(bal1?.eq(totalSupplyToken1 - 1000n)).toBeTruthy();
    })
});
