// SPDX-License-Identifier: Apache-2.0

import expect from 'expect';
import * as anchor from "@project-serum/anchor";
import { Program } from "@project-serum/anchor";
import { Anchor } from "../target/types/anchor";

describe("Anchor", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();

  anchor.setProvider(provider);

  it("test anchor program with anchor tests", async () => {
    // The Account to create.
    const seed = Buffer.from('ChiaSeeds', 'utf8');

    const program = anchor.workspace.Anchor as Program<Anchor>;

    const [myAccount, bump] = await anchor.web3.PublicKey.findProgramAddress([seed], program.programId);


    const { SystemProgram } = anchor.web3;

    // Add your test here.
    const tx = await program.methods.initializeSeed(seed, bump).accounts({
      myAccount: myAccount,
      user: provider.wallet.publicKey,
      systemProgram: SystemProgram.programId,
    }).rpc();

    // string est
    expect(await program.methods.strings("Hello, World!", 102).view()).toBe("input:Hello, World! data:102");

    // sum test
    const sumtest = await program.methods.sum([new anchor.BN(3), new anchor.BN(5), new anchor.BN(7)], new anchor.BN(1)).view();

    expect(sumtest.toNumber()).toBe(1 + 3 + 5 + 7);

    // sector001
    let sector001 = await program.methods.sector001().view();
    expect(sector001.suns.toNumber()).toBe(1);
    expect(sector001.mclass.length).toBe(1);
    expect(sector001.mclass[0]).toMatchObject({ "earth": {} });
    expect(sector001.calldata.toString()).toEqual(Uint8Array.from([48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 48, 49, 50]).toString());

    // has_planet
    sector001.mclass.push({ "mars": {} });

    const has_planet = await program.methods.hasPlanet(sector001, { "mars": {} }).view();

    expect(has_planet).toBe(true)

    // states
    const states = await program.methods.states().accounts({
      myAccount: myAccount
    }).view();

    expect(states.default).toBe(true);
    expect(states.delete).toBe(-102);
    expect(states.fallback.toNumber()).toBe(0xdeadcafebeef);
    expect(states.assembly.toString()).toBe('AddressLookupTab1e1111111111111111111111111');

    // multidimensional
    const arr = await program.methods.multiDimensional([[1, 2, 3], [4, 5, 6], [7, 8, 9], [10, 11, 12]]).view();

    expect(arr).toStrictEqual([[1, 4, 7, 10], [2, 5, 8, 11], [3, 6, 9, 12]]);
  });
});
