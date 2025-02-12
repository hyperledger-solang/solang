import { Transaction } from "@stellar/stellar-base";
/**
 * Miscellaneous utilities.
 *
 * @hideconstructor
 */
export declare class Utils {
    /**
     * Verifies if the current date is within the transaction's timebounds
     *
     * @param {Transaction} transaction The transaction whose timebounds will be validated.
     * @param {number} [gracePeriod=0] An additional window of time that should be considered valid on either end of the transaction's time range.
     *
     * @returns {boolean} Returns true if the current time is within the transaction's [minTime, maxTime] range.
     *
     * @static
     */
    static validateTimebounds(transaction: Transaction, gracePeriod?: number): boolean;
    static sleep(ms: number): Promise<void>;
}
