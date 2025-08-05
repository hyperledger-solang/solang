import * as StellarSdk from '@stellar/stellar-sdk';

export async function call_contract_function(method, server, keypair, contract, ...params) {
    let res = null;
    try {
        let builtTransaction = new StellarSdk.TransactionBuilder(await server.getAccount(keypair.publicKey()), {
            fee: StellarSdk.BASE_FEE,
            networkPassphrase: StellarSdk.Networks.TESTNET,
        })
            .addOperation(contract.call(method, ...params))
            .setTimeout(30)
            .build();

        let preparedTransaction = await server.prepareTransaction(builtTransaction);
        preparedTransaction.sign(keypair);

        let sendResponse = await server.sendTransaction(preparedTransaction);

        if (sendResponse.status === "PENDING") {
            let getResponse = await server.getTransaction(sendResponse.hash);
            while (getResponse.status === "NOT_FOUND") {
                console.log("Waiting for transaction confirmation...");
                await new Promise((resolve) => setTimeout(resolve, 1000));
                getResponse = await server.getTransaction(sendResponse.hash);
            }

            result.raw = getResponse;

            if (getResponse.status === "SUCCESS") {
-stellar-asset-integration-test
                // Return the contract call return value (ScVal)
                res = getResponse.returnValue;
            } else {
                result.status = "ERROR";
                result.error = "Transaction failed: " + (getResponse.resultXdr || JSON.stringify(getResponse));
            }
        } else if (sendResponse.status === "FAILED") {
            result.status = "ERROR";
            if (sendResponse.errorResultXdr) {
                result.error = "Transaction failed: " + sendResponse.errorResultXdr;
            } else {
                result.error = "Transaction failed, unknown error";
            }
            result.raw = sendResponse;
        } else {
            result.status = "ERROR";
            result.error = "Unknown sendResponse status: " + sendResponse.status;
            result.raw = sendResponse;
        }
    } catch (err) {
        result.status = "ERROR";
        result.error = "Exception: " + err.toString();
    }

    return result;
}

// Uses StellarSdk.humanizeEvents to extract log messages from diagnosticEventsXdr
export function extractLogMessagesFromDiagnosticEvents(raw) {
  if (!raw.diagnosticEventsXdr || !Array.isArray(raw.diagnosticEventsXdr)) return [];
  try {
    // humanizeEvents expects an array of DiagnosticEvent XDRs (base64 or already parsed)
    const events = StellarSdk.humanizeEvents(raw.diagnosticEventsXdr);
    // Find log events and collect their messages
    return events
      .filter(ev => ev.type === "diagnostic" && ev.topics && ev.topics.includes("log"))
      .map(ev => (Array.isArray(ev.data) ? ev.data.join(" ") : ev.data));
  } catch (e) {
    return [];
  }
}

// Helper to stringify BigInt values (for assertion error output)
export function toSafeJson(obj) {
  return JSON.stringify(obj, (_key, value) =>
    typeof value === 'bigint' ? value.toString() : value,
  2);
}
