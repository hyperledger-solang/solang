"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.parseRawEvents = parseRawEvents;
exports.parseRawLedgerEntries = parseRawLedgerEntries;
exports.parseRawSendTransaction = parseRawSendTransaction;
exports.parseRawSimulation = parseRawSimulation;
exports.parseRawTransactions = parseRawTransactions;
exports.parseTransactionInfo = parseTransactionInfo;
var _stellarBase = require("@stellar/stellar-base");
var _api = require("./api");
function _typeof(o) { "@babel/helpers - typeof"; return _typeof = "function" == typeof Symbol && "symbol" == typeof Symbol.iterator ? function (o) { return typeof o; } : function (o) { return o && "function" == typeof Symbol && o.constructor === Symbol && o !== Symbol.prototype ? "symbol" : typeof o; }, _typeof(o); }
function ownKeys(e, r) { var t = Object.keys(e); if (Object.getOwnPropertySymbols) { var o = Object.getOwnPropertySymbols(e); r && (o = o.filter(function (r) { return Object.getOwnPropertyDescriptor(e, r).enumerable; })), t.push.apply(t, o); } return t; }
function _objectSpread(e) { for (var r = 1; r < arguments.length; r++) { var t = null != arguments[r] ? arguments[r] : {}; r % 2 ? ownKeys(Object(t), !0).forEach(function (r) { _defineProperty(e, r, t[r]); }) : Object.getOwnPropertyDescriptors ? Object.defineProperties(e, Object.getOwnPropertyDescriptors(t)) : ownKeys(Object(t)).forEach(function (r) { Object.defineProperty(e, r, Object.getOwnPropertyDescriptor(t, r)); }); } return e; }
function _defineProperty(e, r, t) { return (r = _toPropertyKey(r)) in e ? Object.defineProperty(e, r, { value: t, enumerable: !0, configurable: !0, writable: !0 }) : e[r] = t, e; }
function _toPropertyKey(t) { var i = _toPrimitive(t, "string"); return "symbol" == _typeof(i) ? i : i + ""; }
function _toPrimitive(t, r) { if ("object" != _typeof(t) || !t) return t; var e = t[Symbol.toPrimitive]; if (void 0 !== e) { var i = e.call(t, r || "default"); if ("object" != _typeof(i)) return i; throw new TypeError("@@toPrimitive must return a primitive value."); } return ("string" === r ? String : Number)(t); }
function parseRawSendTransaction(raw) {
  var errorResultXdr = raw.errorResultXdr,
    diagnosticEventsXdr = raw.diagnosticEventsXdr;
  delete raw.errorResultXdr;
  delete raw.diagnosticEventsXdr;
  if (errorResultXdr) {
    return _objectSpread(_objectSpread(_objectSpread({}, raw), diagnosticEventsXdr !== undefined && diagnosticEventsXdr.length > 0 && {
      diagnosticEvents: diagnosticEventsXdr.map(function (evt) {
        return _stellarBase.xdr.DiagnosticEvent.fromXDR(evt, 'base64');
      })
    }), {}, {
      errorResult: _stellarBase.xdr.TransactionResult.fromXDR(errorResultXdr, 'base64')
    });
  }
  return _objectSpread({}, raw);
}
function parseTransactionInfo(raw) {
  var meta = _stellarBase.xdr.TransactionMeta.fromXDR(raw.resultMetaXdr, 'base64');
  var info = {
    ledger: raw.ledger,
    createdAt: raw.createdAt,
    applicationOrder: raw.applicationOrder,
    feeBump: raw.feeBump,
    envelopeXdr: _stellarBase.xdr.TransactionEnvelope.fromXDR(raw.envelopeXdr, 'base64'),
    resultXdr: _stellarBase.xdr.TransactionResult.fromXDR(raw.resultXdr, 'base64'),
    resultMetaXdr: meta
  };
  if (meta.switch() === 3 && meta.v3().sorobanMeta() !== null) {
    var _meta$v3$sorobanMeta;
    info.returnValue = (_meta$v3$sorobanMeta = meta.v3().sorobanMeta()) === null || _meta$v3$sorobanMeta === void 0 ? void 0 : _meta$v3$sorobanMeta.returnValue();
  }
  if ('diagnosticEventsXdr' in raw && raw.diagnosticEventsXdr) {
    info.diagnosticEventsXdr = raw.diagnosticEventsXdr.map(function (diagnosticEvent) {
      return _stellarBase.xdr.DiagnosticEvent.fromXDR(diagnosticEvent, 'base64');
    });
  }
  return info;
}
function parseRawTransactions(r) {
  return _objectSpread({
    status: r.status,
    txHash: r.txHash
  }, parseTransactionInfo(r));
}
function parseRawEvents(raw) {
  var _raw$events;
  return {
    latestLedger: raw.latestLedger,
    cursor: raw.cursor,
    events: ((_raw$events = raw.events) !== null && _raw$events !== void 0 ? _raw$events : []).map(function (evt) {
      var clone = _objectSpread({}, evt);
      delete clone.contractId;
      return _objectSpread(_objectSpread(_objectSpread({}, clone), evt.contractId !== '' && {
        contractId: new _stellarBase.Contract(evt.contractId)
      }), {}, {
        topic: evt.topic.map(function (topic) {
          return _stellarBase.xdr.ScVal.fromXDR(topic, 'base64');
        }),
        value: _stellarBase.xdr.ScVal.fromXDR(evt.value, 'base64')
      });
    })
  };
}
function parseRawLedgerEntries(raw) {
  var _raw$entries;
  return {
    latestLedger: raw.latestLedger,
    entries: ((_raw$entries = raw.entries) !== null && _raw$entries !== void 0 ? _raw$entries : []).map(function (rawEntry) {
      if (!rawEntry.key || !rawEntry.xdr) {
        throw new TypeError("invalid ledger entry: ".concat(JSON.stringify(rawEntry)));
      }
      return _objectSpread({
        lastModifiedLedgerSeq: rawEntry.lastModifiedLedgerSeq,
        key: _stellarBase.xdr.LedgerKey.fromXDR(rawEntry.key, 'base64'),
        val: _stellarBase.xdr.LedgerEntryData.fromXDR(rawEntry.xdr, 'base64')
      }, rawEntry.liveUntilLedgerSeq !== undefined && {
        liveUntilLedgerSeq: rawEntry.liveUntilLedgerSeq
      });
    })
  };
}
function parseSuccessful(sim, partial) {
  var _sim$results$length, _sim$results, _sim$stateChanges$len, _sim$stateChanges, _sim$stateChanges2;
  var success = _objectSpread(_objectSpread(_objectSpread({}, partial), {}, {
    transactionData: new _stellarBase.SorobanDataBuilder(sim.transactionData),
    minResourceFee: sim.minResourceFee
  }, ((_sim$results$length = (_sim$results = sim.results) === null || _sim$results === void 0 ? void 0 : _sim$results.length) !== null && _sim$results$length !== void 0 ? _sim$results$length : 0 > 0) && {
    result: sim.results.map(function (row) {
      var _row$auth;
      return {
        auth: ((_row$auth = row.auth) !== null && _row$auth !== void 0 ? _row$auth : []).map(function (entry) {
          return _stellarBase.xdr.SorobanAuthorizationEntry.fromXDR(entry, 'base64');
        }),
        retval: row.xdr ? _stellarBase.xdr.ScVal.fromXDR(row.xdr, 'base64') : _stellarBase.xdr.ScVal.scvVoid()
      };
    })[0]
  }), ((_sim$stateChanges$len = (_sim$stateChanges = sim.stateChanges) === null || _sim$stateChanges === void 0 ? void 0 : _sim$stateChanges.length) !== null && _sim$stateChanges$len !== void 0 ? _sim$stateChanges$len : 0 > 0) && {
    stateChanges: (_sim$stateChanges2 = sim.stateChanges) === null || _sim$stateChanges2 === void 0 ? void 0 : _sim$stateChanges2.map(function (entryChange) {
      return {
        type: entryChange.type,
        key: _stellarBase.xdr.LedgerKey.fromXDR(entryChange.key, 'base64'),
        before: entryChange.before ? _stellarBase.xdr.LedgerEntry.fromXDR(entryChange.before, 'base64') : null,
        after: entryChange.after ? _stellarBase.xdr.LedgerEntry.fromXDR(entryChange.after, 'base64') : null
      };
    })
  });
  if (!sim.restorePreamble || sim.restorePreamble.transactionData === '') {
    return success;
  }
  return _objectSpread(_objectSpread({}, success), {}, {
    restorePreamble: {
      minResourceFee: sim.restorePreamble.minResourceFee,
      transactionData: new _stellarBase.SorobanDataBuilder(sim.restorePreamble.transactionData)
    }
  });
}
function parseRawSimulation(sim) {
  var _sim$events$map, _sim$events;
  var looksRaw = _api.Api.isSimulationRaw(sim);
  if (!looksRaw) {
    return sim;
  }
  var base = {
    _parsed: true,
    id: sim.id,
    latestLedger: sim.latestLedger,
    events: (_sim$events$map = (_sim$events = sim.events) === null || _sim$events === void 0 ? void 0 : _sim$events.map(function (evt) {
      return _stellarBase.xdr.DiagnosticEvent.fromXDR(evt, 'base64');
    })) !== null && _sim$events$map !== void 0 ? _sim$events$map : []
  };
  if (typeof sim.error === 'string') {
    return _objectSpread(_objectSpread({}, base), {}, {
      error: sim.error
    });
  }
  return parseSuccessful(sim, base);
}