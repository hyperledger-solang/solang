import expect from 'expect';
import { gasLimit, createConnection, deploy, transaction, aliceKeypair, daveKeypair } from './index';
import { ContractPromise } from '@polkadot/api-contract';
import { ApiPromise } from '@polkadot/api';
import { KeyringPair } from '@polkadot/keyring/types';
import { keccakAsU8a, keccakAsHex } from '@polkadot/util-crypto';

const TOTAL_SUPPLY = BigInt(10000e18);
const TEST_AMOUNT = BigInt(10e18);
const MAX_UINT256 = BigInt(0x7fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff);

describe('Deploy UniswapV2ERC20 contract and test', () => {
    let conn: ApiPromise;
    let token: ContractPromise;
    let alice: KeyringPair;
    let dave: KeyringPair;

    beforeEach(async function () {
        conn = await createConnection();

        alice = aliceKeypair();
        dave = daveKeypair();

        let deploy_contract = await deploy(conn, alice, 'ERC20.contract', TOTAL_SUPPLY);

        token = new ContractPromise(conn, deploy_contract.abi, deploy_contract.address);
    });

    afterEach(async function () {
        await conn.disconnect();
    });

    it('name, symbol, decimals, totalSupply, balanceOf, DOMAIN_SEPARATOR, PERMIT_TYPEHASH', async () => {
        const { output: name } = await token.query.name(alice.address, {});
        expect(name?.toJSON()).toEqual('Uniswap V2')
        const { output: symbol } = await token.query.symbol(alice.address, {});
        expect(symbol?.toJSON()).toEqual('UNI-V2')
        const { output: decimals } = await token.query.decimals(alice.address, {});
        expect(decimals?.toJSON()).toEqual(18)
        const { output: totalSupply } = await token.query.totalSupply(alice.address, {});
        //console.log(`total supply: ${totalSupply?.toHuman()}`);
        expect(totalSupply?.eq(TOTAL_SUPPLY)).toBeTruthy();
        const { output: bal } = await token.query.balanceOf(alice.address, {}, alice.address);
        expect(bal?.eq(TOTAL_SUPPLY)).toBeTruthy();

        const { output: domain_seperator } = await token.query.domainSeparator(alice.address, {});

        let expected = keccakAsHex(Buffer.concat([
            keccakAsU8a('EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)'),
            keccakAsU8a(name!.toString()),
            keccakAsU8a('1'),
            Buffer.from('0100000000000000000000000000000000000000000000000000000000000000', 'hex'),
            Buffer.from(token.address.buffer),
        ]));

        //console.log(`domain_separator: ${domain_seperator} ${expected}`);
        expect(domain_seperator?.eq(expected)).toBeTruthy();

        const { output: permit_typehash } = await token.query.permitTypehash(alice.address, {});
        expect(permit_typehash?.eq('0x6e71edae12b1b97f4d1f60370fef10105fa2faae0126114a169c64845d6126c9')).toBeTruthy();
    })

    it('approve', async () => {
        let tx = token.tx.approve({ gasLimit }, dave.address, TEST_AMOUNT);
        await transaction(tx, alice);

        let { output } = await token.query.allowance(alice.address, {}, alice.address, dave.address);
        expect(output?.eq(TEST_AMOUNT)).toBeTruthy();
    })

    it('transfer', async () => {
        let tx = token.tx.transfer({ gasLimit }, dave.address, TEST_AMOUNT);
        await transaction(tx, alice);

        const { output: aliceBal } = await token.query.balanceOf(alice.address, {}, alice.address);
        expect(aliceBal?.eq(TOTAL_SUPPLY - TEST_AMOUNT)).toBeTruthy();
        const { output: daveBal } = await token.query.balanceOf(alice.address, {}, dave.address);
        expect(daveBal?.eq(TEST_AMOUNT)).toBeTruthy();
    })

    // it('transfer:fail', async () => {
    //     await expect(token.transfer(other.address, TOTAL_SUPPLY.add(1))).to.be.reverted // ds-math-sub-underflow
    //     await expect(token.connect(other).transfer(wallet.address, 1)).to.be.reverted // ds-math-sub-underflow
    // })

    it('transferFrom', async () => {
        let tx = token.tx.approve({ gasLimit }, dave.address, TEST_AMOUNT);
        await transaction(tx, alice);

        tx = token.tx.transferFrom({ gasLimit }, alice.address, dave.address, TEST_AMOUNT);
        await transaction(tx, dave);

        const { output: allowance } = await token.query.allowance(alice.address, {}, alice.address, dave.address);
        expect(allowance?.eq(0)).toBeTruthy();
        const { output: aliceBal } = await token.query.balanceOf(alice.address, {}, alice.address);
        expect(aliceBal?.eq(TOTAL_SUPPLY - TEST_AMOUNT)).toBeTruthy();
        const { output: daveBal } = await token.query.balanceOf(alice.address, {}, dave.address);
        expect(daveBal?.eq(TEST_AMOUNT)).toBeTruthy();
    })

    it('transferFrom:max', async () => {
        let tx = token.tx.approve({ gasLimit }, dave.address, MAX_UINT256);
        await transaction(tx, alice);

        tx = token.tx.transferFrom({ gasLimit }, alice.address, dave.address, TEST_AMOUNT);
        await transaction(tx, dave);

        const { output: allowance } = await token.query.allowance(alice.address, {}, alice.address, dave.address);
        expect(allowance?.eq(MAX_UINT256 - TEST_AMOUNT)).toBeTruthy();
        const { output: aliceBal } = await token.query.balanceOf(alice.address, {}, alice.address);
        expect(aliceBal?.eq(TOTAL_SUPPLY - TEST_AMOUNT)).toBeTruthy();
        const { output: daveBal } = await token.query.balanceOf(alice.address, {}, dave.address);
        expect(daveBal?.eq(TEST_AMOUNT)).toBeTruthy();
    })
});
