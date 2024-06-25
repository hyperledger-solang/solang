import * as StellarSdk from '@stellar/stellar-sdk';



export async function call_contract_function(method, server, keypair, contract) {

    let res;
    let builtTransaction = new StellarSdk.TransactionBuilder(await server.getAccount(keypair.publicKey()), {
      fee: StellarSdk.BASE_FEE,
      networkPassphrase: StellarSdk.Networks.TESTNET,
    }).addOperation(contract.call(method)).setTimeout(30).build();
  
    let preparedTransaction = await server.prepareTransaction(builtTransaction);
  
    // Sign the transaction with the source account's keypair.
    preparedTransaction.sign(keypair);
  
    try {
      let sendResponse = await server.sendTransaction(preparedTransaction);
      if (sendResponse.status === "PENDING") {
        let getResponse = await server.getTransaction(sendResponse.hash);
        // Poll `getTransaction` until the status is not "NOT_FOUND"
        while (getResponse.status === "NOT_FOUND") {
          console.log("Waiting for transaction confirmation...");
          // See if the transaction is complete
          getResponse = await server.getTransaction(sendResponse.hash);
          // Wait one second
          await new Promise((resolve) => setTimeout(resolve, 1000));
        }
  
        if (getResponse.status === "SUCCESS") {
          // Make sure the transaction's resultMetaXDR is not empty
          if (!getResponse.resultMetaXdr) {
            throw "Empty resultMetaXDR in getTransaction response";
          }
          // Find the return value from the contract and return it
          let transactionMeta = getResponse.resultMetaXdr;
          let returnValue = transactionMeta.v3().sorobanMeta().returnValue();
          console.log(`Transaction result: ${returnValue.value()}`);
          res = returnValue.value();
        } else {
          throw `Transaction failed: ${getResponse.resultXdr}`;
        }
      } else {
        throw sendResponse.errorResultXdr;
      }
    } catch (err) {
      // Catch and report any errors we've thrown
      console.log("Sending transaction failed");
      console.log(err);
    }
    return res;
}