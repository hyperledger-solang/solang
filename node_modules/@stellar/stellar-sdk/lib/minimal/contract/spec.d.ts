import type { JSONSchema7 } from "json-schema";
import { xdr } from "@stellar/stellar-base";
export interface Union<T> {
    tag: string;
    values?: T;
}
/**
 * Provides a ContractSpec class which can contains the XDR types defined by the contract.
 * This allows the class to be used to convert between native and raw `xdr.ScVal`s.
 *
 * Constructs a new ContractSpec from an array of XDR spec entries.
 *
 * @memberof module:contract
 * @param {xdr.ScSpecEntry[] | string[]} entries the XDR spec entries
 * @throws {Error} if entries is invalid
 *
 * @example
 * const specEntries = [...]; // XDR spec entries of a smart contract
 * const contractSpec = new ContractSpec(specEntries);
 *
 * // Convert native value to ScVal
 * const args = {
 *   arg1: 'value1',
 *   arg2: 1234
 * };
 * const scArgs = contractSpec.funcArgsToScVals('funcName', args);
 *
 * // Call contract
 * const resultScv = await callContract(contractId, 'funcName', scArgs);
 *
 * // Convert result ScVal back to native value
 * const result = contractSpec.funcResToNative('funcName', resultScv);
 *
 * console.log(result); // {success: true}
 */
export declare class Spec {
    /**
     * The XDR spec entries.
     */
    entries: xdr.ScSpecEntry[];
    constructor(entries: xdr.ScSpecEntry[] | string[]);
    /**
     * Gets the XDR functions from the spec.
     * @returns {xdr.ScSpecFunctionV0[]} all contract functions
     */
    funcs(): xdr.ScSpecFunctionV0[];
    /**
     * Gets the XDR function spec for the given function name.
     *
     * @param {string} name the name of the function
     * @returns {xdr.ScSpecFunctionV0} the function spec
     *
     * @throws {Error} if no function with the given name exists
     */
    getFunc(name: string): xdr.ScSpecFunctionV0;
    /**
     * Converts native JS arguments to ScVals for calling a contract function.
     *
     * @param {string} name the name of the function
     * @param {object} args the arguments object
     * @returns {xdr.ScVal[]} the converted arguments
     *
     * @throws {Error} if argument is missing or incorrect type
     *
     * @example
     * const args = {
     *   arg1: 'value1',
     *   arg2: 1234
     * };
     * const scArgs = contractSpec.funcArgsToScVals('funcName', args);
     */
    funcArgsToScVals(name: string, args: object): xdr.ScVal[];
    /**
     * Converts the result ScVal of a function call to a native JS value.
     *
     * @param {string} name the name of the function
     * @param {xdr.ScVal | string} val_or_base64 the result ScVal or base64 encoded string
     * @returns {any} the converted native value
     *
     * @throws {Error} if return type mismatch or invalid input
     *
     * @example
     * const resultScv = 'AAA=='; // Base64 encoded ScVal
     * const result = contractSpec.funcResToNative('funcName', resultScv);
     */
    funcResToNative(name: string, val_or_base64: xdr.ScVal | string): any;
    /**
     * Finds the XDR spec entry for the given name.
     *
     * @param {string} name the name to find
     * @returns {xdr.ScSpecEntry} the entry
     *
     * @throws {Error} if no entry with the given name exists
     */
    findEntry(name: string): xdr.ScSpecEntry;
    /**
     * Converts a native JS value to an ScVal based on the given type.
     *
     * @param {any} val the native JS value
     * @param {xdr.ScSpecTypeDef} [ty] the expected type
     * @returns {xdr.ScVal} the converted ScVal
     *
     * @throws {Error} if value cannot be converted to the given type
     */
    nativeToScVal(val: any, ty: xdr.ScSpecTypeDef): xdr.ScVal;
    private nativeToUdt;
    private nativeToUnion;
    private nativeToStruct;
    private nativeToEnum;
    /**
     * Converts an base64 encoded ScVal back to a native JS value based on the given type.
     *
     * @param {string} scv the base64 encoded ScVal
     * @param {xdr.ScSpecTypeDef} typeDef the expected type
     * @returns {any} the converted native JS value
     *
     * @throws {Error} if ScVal cannot be converted to the given type
     */
    scValStrToNative<T>(scv: string, typeDef: xdr.ScSpecTypeDef): T;
    /**
     * Converts an ScVal back to a native JS value based on the given type.
     *
     * @param {xdr.ScVal} scv the ScVal
     * @param {xdr.ScSpecTypeDef} typeDef the expected type
     * @returns {any} the converted native JS value
     *
     * @throws {Error} if ScVal cannot be converted to the given type
     */
    scValToNative<T>(scv: xdr.ScVal, typeDef: xdr.ScSpecTypeDef): T;
    private scValUdtToNative;
    private unionToNative;
    private structToNative;
    private enumToNative;
    /**
     * Gets the XDR error cases from the spec.
     *
     * @returns {xdr.ScSpecFunctionV0[]} all contract functions
     *
     */
    errorCases(): xdr.ScSpecUdtErrorEnumCaseV0[];
    /**
     * Converts the contract spec to a JSON schema.
     *
     * If `funcName` is provided, the schema will be a reference to the function schema.
     *
     * @param {string} [funcName] the name of the function to convert
     * @returns {JSONSchema7} the converted JSON schema
     *
     * @throws {Error} if the contract spec is invalid
     */
    jsonSchema(funcName?: string): JSONSchema7;
}
