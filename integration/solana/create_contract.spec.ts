import { Signer } from '@solana/web3.js';
import expect from 'expect';
import { Contract, createProgramDerivedAddress, ProgramDerivedAddress } from '@solana/solidity';
import { loadContract } from './setup';

describe('ChildContract', function () {
    this.timeout(150000);

    let contract: Contract;
    let storage: Signer;

    let childPDA: ProgramDerivedAddress;

    before(async function () {
        ({ contract, storage } = await loadContract('creator', 'creator.abi'));
    });

    // FIXME:
    // Test disabled in https://github.com/hyperledger/solang/pull/1039
    // shoulde be fixed by https://github.com/solana-labs/solana-solidity.js/pull/42
    xit('Creates child contract', async function () {
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