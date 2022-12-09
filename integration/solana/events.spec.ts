// SPDX-License-Identifier: Apache-2.0

import expect from 'expect';
import { loadContract } from './setup';
import { deserialize, field } from "@dao-xyz/borsh";
import * as sha256 from "fast-sha256";

describe('Test events', function () {
    this.timeout(500000);
    const LOG_DATA_PREFIX = 'Program data: ';
    class First {
        @field({ type: "u64" })
        discriminator: bigint

        @field({ type: "u32" })
        a: number

        @field({ type: "bool" })
        b: boolean

        @field({ type: "string" })
        c: string

        constructor(data?: { discriminator: bigint, a: number, b: boolean, c: string }) {
            if (data) {
                this.discriminator = data.discriminator;
                this.a = data.a;
                this.b = data.b;
                this.c = data.c;
            } else {
                this.discriminator = BigInt(0);
                this.a = 0;
                this.b = false;
                this.c = "";
            }
        }
    }

    class Second {
        @field({ type: "u64" })
        discriminator: bigint

        @field({ type: "u32" })
        a: number

        @field({ type: "string" })
        b: string

        @field({ type: "string" })
        c: string

        constructor(data?: { discriminator: bigint, a: number, b: string, c: string }) {
            if (data) {
                this.discriminator = data.discriminator;
                this.a = data.a;
                this.b = data.b;
                this.c = data.c;
            } else {
                this.discriminator = BigInt(0);
                this.a = 0;
                this.b = "";
                this.c = "";
            }
        }
    }

    function changeEndiannes(bytes: string): string {
        let result: string = "";
        for (let i = 0; i < bytes.length; i += 2) {
            result += bytes.substring(bytes.length - i - 2, bytes.length - i);
        }
        return result;
    }

    it('events', async function () {
        const { contract, connection, program, storage } = await loadContract('MyContractEvents');

        let res = await contract.functions.test({ simulate: true });

        expect(res.result).toBeNull();
        let eventData: Buffer[] = [];
        for (let programLog of res.logs) {
            if (programLog.startsWith(LOG_DATA_PREFIX)) {
                const fields = programLog.slice(LOG_DATA_PREFIX.length).split(' ');
                if (fields.length == 1) {
                    eventData.push(Buffer.from(fields[0], 'base64'));
                }
            }
        }
        expect(eventData.length).toBe(2);
        let event_1 = deserialize(eventData[0], First);

        let discriminator_image = "event:First";
        let hasher = new sha256.Hash();
        hasher.update(new TextEncoder().encode(discriminator_image));
        let result = hasher.digest();
        let received_discriminator = changeEndiannes(event_1.discriminator.toString(16));
        expect(received_discriminator).toBe(Buffer.from(result.slice(0, 8)).toString('hex'));
        expect(event_1.a).toBe(102);
        expect(event_1.b).toBe(true);
        expect(event_1.c).toBe("foobar");

        let event_2 = deserialize(eventData[1], Second);

        discriminator_image = "event:Second";
        hasher = new sha256.Hash();
        hasher.update(new TextEncoder().encode(discriminator_image));
        result = hasher.digest();
        received_discriminator = changeEndiannes(event_2.discriminator.toString(16));
        expect(received_discriminator).toBe(Buffer.from(result.slice(0, 8)).toString('hex'));
        expect(event_2.a).toBe(500332);
        expect(event_2.b).toBe("ABCD");
        expect(event_2.c).toBe("CAFE0123");
    });
});
