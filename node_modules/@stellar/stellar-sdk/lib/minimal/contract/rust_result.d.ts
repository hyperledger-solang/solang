/**
 * A minimal implementation of Rust's `Result` type. Used for contract
 * methods that return Results, to maintain their distinction from methods
 * that simply either return a value or throw.
 *
 * #### Why is this needed?
 *
 * This is used by {@link module:contract.Spec | `ContractSpec`} and
 * {@link module:contract.AssembledTransaction | `AssembledTransaction`} when
 * parsing values return by contracts.
 *
 * Contract methods can be implemented to return simple values, in which case
 * they can also throw errors. This matches JavaScript's most idiomatic
 * workflow, using `try...catch` blocks.
 *
 * But Rust also gives the flexibility of returning `Result` types. And Soroban
 * contracts further support this with the `#[contracterror]` macro. Should
 * JavaScript calls to such methods ignore all of that, and just flatten this
 * extra info down to the same `try...catch` flow as other methods? We're not
 * sure.
 *
 * For now, we've added this minimal implementation of Rust's `Result` logic,
 * which exports the `Result` interface and its associated implementations,
 * `Ok` and `Err`. This allows `ContractSpec` and `AssembledTransaction` to
 * work together to duplicate the contract's Rust logic, always returning
 * `Result` types for contract methods that are implemented to do so.
 *
 * In the future, if this feels too un-idiomatic for JavaScript, we can always
 * remove this and flatten all JS calls to `try...catch`. Easier to remove this
 * logic later than it would be to add it.
 *
 * @memberof module:contract
 */
export interface Result<T, E extends ErrorMessage = ErrorMessage> {
    unwrap(): T;
    unwrapErr(): E;
    isOk(): boolean;
    isErr(): boolean;
}
/**
 * Error interface containing the error message. Matches Rust's implementation.
 * Part of implementing {@link module:contract.Result | Result}, a minimal
 * implementation of Rust's `Result` type. Used for contract methods that return
 * Results, to maintain their distinction from methods that simply either return
 * a value or throw.
 *
 * @memberof module:contract
 */
export interface ErrorMessage {
    message: string;
}
/**
 * Part of implementing {@link module:contract.Result | Result}, a minimal
 * implementation of Rust's `Result` type. Used for contract methods that return
 * Results, to maintain their distinction from methods that simply either return
 * a value or throw.
 * @private
 */
export declare class Ok<T> implements Result<T, never> {
    readonly value: T;
    constructor(value: T);
    unwrapErr(): never;
    unwrap(): T;
    isOk(): boolean;
    isErr(): boolean;
}
/**
 * Part of implementing {@link module:contract.Result | Result}, a minimal
 * implementation of Rust's `Result` type. Used for contract methods that return
 * Results, to maintain their distinction from methods that simply either return
 * a value or throw.
 * @private
 */
export declare class Err<E extends ErrorMessage> implements Result<never, E> {
    readonly error: E;
    constructor(error: E);
    unwrapErr(): E;
    unwrap(): never;
    isOk(): boolean;
    isErr(): boolean;
}
