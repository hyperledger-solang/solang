import { Signer } from '@solana/web3.js';
import expect from 'expect';
import { Contract, createProgramDerivedAddress, ProgramDerivedAddress } from '@solana/solidity';
import { loadContract } from './utils';

describe('ChildContract', () => {
    let contract: Contract;
    let storage: Signer;

    let childPDA: ProgramDerivedAddress;

    before(async function () {
        this.timeout(150000);
        ({ contract, storage } = await loadContract('creator', 'creator.abi'));
    });

    it('Creates child contract', async function () {
        childPDA = await createProgramDerivedAddress(contract.program);

        const { logs } = await contract.functions.create_child({
            accounts: [contract.program],
            programDerivedAddresses: [childPDA],
            signers: [storage],
        });

        expect(logs.toString()).toContain('In child constructor');

        const info = await contract.connection.getAccountInfo(childPDA.address);
        console.log('info: ' + info);
    });
});