"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.Api = void 0;
var Api;
(function (_Api) {
  var GetTransactionStatus = function (GetTransactionStatus) {
    GetTransactionStatus["SUCCESS"] = "SUCCESS";
    GetTransactionStatus["NOT_FOUND"] = "NOT_FOUND";
    GetTransactionStatus["FAILED"] = "FAILED";
    return GetTransactionStatus;
  }({});
  _Api.GetTransactionStatus = GetTransactionStatus;
  function isSimulationError(sim) {
    return 'error' in sim;
  }
  _Api.isSimulationError = isSimulationError;
  function isSimulationSuccess(sim) {
    return 'transactionData' in sim;
  }
  _Api.isSimulationSuccess = isSimulationSuccess;
  function isSimulationRestore(sim) {
    return isSimulationSuccess(sim) && 'restorePreamble' in sim && !!sim.restorePreamble.transactionData;
  }
  _Api.isSimulationRestore = isSimulationRestore;
  function isSimulationRaw(sim) {
    return !sim._parsed;
  }
  _Api.isSimulationRaw = isSimulationRaw;
})(Api || (exports.Api = Api = {}));