import * as StellarSdk from '@stellar/stellar-sdk';

export async function call_contract_function(method, server, keypair, contract) {
    let res = null;

    try {
        let builtTransaction = new StellarSdk.TransactionBuilder(await server.getAccount(keypair.publicKey()), {
            fee: StellarSdk.BASE_FEE,
            networkPassphrase: StellarSdk.Networks.TESTNET,
        }).addOperation(contract.call(method)).setTimeout(30).build();

        let preparedTransaction = await server.prepareTransaction(builtTransaction);

        // Sign the transaction with the source account's keypair.
        preparedTransaction.sign(keypair);

        let sendResponse = await server.sendTransaction(preparedTransaction);

        if (sendResponse.status === "PENDING") {
            let getResponse = await server.getTransaction(sendResponse.hash);
            // Poll `getTransaction` until the status is not "NOT_FOUND"
            while (getResponse.status === "NOT_FOUND") {
                console.log("Waiting for transaction confirmation...");
                // Wait one second
                await new Promise((resolve) => setTimeout(resolve, 1000));
                // See if the transaction is complete
                getResponse = await server.getTransaction(sendResponse.hash);
            }

            if (getResponse.status === "SUCCESS") {
                // Ensure the transaction's resultMetaXDR is not empty
                if (!getResponse.resultMetaXdr) {
                    throw "Empty resultMetaXDR in getTransaction response";
                }
                // Extract and return the return value from the contract
                let transactionMeta = getResponse.resultMetaXdr;
                let returnValue = transactionMeta.v3().sorobanMeta().returnValue();
                console.log(`Transaction result: ${returnValue.value()}`);
                res = returnValue.value();
            } else {
                throw `Transaction failed: ${getResponse.resultXdr}`;
            }
        } else if (sendResponse.status === "FAILED") {
            // Handle expected failure and return the error message
            if (sendResponse.errorResultXdr) {
                const errorXdr = StellarSdk.xdr.TransactionResult.fromXDR(sendResponse.errorResultXdr, 'base64');
                const errorRes = errorXdr.result().results()[0].tr().invokeHostFunctionResult().code().value;
                console.log(`Transaction error: ${errorRes}`);
                res = errorRes;
            } else {
                throw "Transaction failed but no errorResultXdr found";
            }
        } else {
            throw sendResponse.errorResultXdr;
        }
    } catch (err) {
        // Return the error as a string instead of failing the test
        console.log("Transaction processing failed");
        console.log(err);
        res = err.toString();
    }

    return res;
}
