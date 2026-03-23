import * as StellarSdk from '@stellar/stellar-sdk';

// Utility to decode Soroban return value (ScVal) to native JS
function decodeReturnValue(scval) {
    if (!scval) return undefined;
    if (StellarSdk.scValToNative) {
        return StellarSdk.scValToNative(scval);
    }
    if (scval._value) return scval._value;
    return scval;
}

async function buildContractCallTransaction(server, keypair, contract, method, params) {
    return new StellarSdk.TransactionBuilder(await server.getAccount(keypair.publicKey()), {
        fee: StellarSdk.BASE_FEE,
        networkPassphrase: StellarSdk.Networks.TESTNET,
    })
        .addOperation(contract.call(method, ...params))
        .setTimeout(30)
        .build();
}

function extractSimulationRetval(simulation) {
    const candidate = simulation?.result?.retval ?? simulation?.results?.[0]?.retval;
    if (!candidate) return null;

    if (candidate && typeof candidate.switch === 'function') return candidate;
    if (typeof candidate === 'string') {
        return StellarSdk.xdr.ScVal.fromXDR(candidate, 'base64');
    }
    if (candidate && typeof candidate.toXDR === 'function') return candidate;
    return null;
}

export async function call_contract_view(method, server, keypair, contract, ...params) {
    const result = {
        status: null,
        returnValue: null,
        error: null,
        raw: null,
    };

    try {
        const builtTransaction = await buildContractCallTransaction(
            server,
            keypair,
            contract,
            method,
            params
        );
        const simulation = await server.simulateTransaction(builtTransaction);
        result.raw = simulation;

        const simulationError =
            simulation?.error ??
            simulation?.result?.error ??
            simulation?.results?.[0]?.error;

        if (simulationError) {
            result.status = 'ERROR';
            result.error = `Simulation failed: ${JSON.stringify(simulationError)}`;
            return result;
        }

        const retval = extractSimulationRetval(simulation);
        result.status = 'SUCCESS';
        if (retval) {
            result.returnValue = decodeReturnValue(retval);
        }
    } catch (err) {
        result.status = 'ERROR';
        result.error = `Exception: ${err.toString()}`;
    }

    return result;
}

export async function call_contract_function(method, server, keypair, contract, ...params) {
    const result = {
        status: null,
        returnValue: null,
        error: null,
        raw: null,
    };

    try {
        const builtTransaction = await buildContractCallTransaction(
            server,
            keypair,
            contract,
            method,
            params
        );
        const preparedTransaction = await server.prepareTransaction(builtTransaction);
        preparedTransaction.sign(keypair);

        const sendResponse = await server.sendTransaction(preparedTransaction);

        if (sendResponse.status === "PENDING") {
            const finalResponse = await server.pollTransaction(sendResponse.hash);
            result.raw = finalResponse;

            if (finalResponse.status === "SUCCESS") {
                result.status = "SUCCESS";
                if (finalResponse.returnValue) {
                    try {
                        result.returnValue = decodeReturnValue(finalResponse.returnValue);
                    } catch (e) {
                        result.error = "Failed to decode returnValue: " + e.toString();
                    }
                }
            } else {
                result.status = "ERROR";
                result.error = "Transaction failed: " + (finalResponse.resultXdr || JSON.stringify(finalResponse));
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
