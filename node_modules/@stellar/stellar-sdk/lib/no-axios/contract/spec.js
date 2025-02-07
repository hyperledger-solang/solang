"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.Spec = void 0;
var _stellarBase = require("@stellar/stellar-base");
var _rust_result = require("./rust_result");
function ownKeys(e, r) { var t = Object.keys(e); if (Object.getOwnPropertySymbols) { var o = Object.getOwnPropertySymbols(e); r && (o = o.filter(function (r) { return Object.getOwnPropertyDescriptor(e, r).enumerable; })), t.push.apply(t, o); } return t; }
function _objectSpread(e) { for (var r = 1; r < arguments.length; r++) { var t = null != arguments[r] ? arguments[r] : {}; r % 2 ? ownKeys(Object(t), !0).forEach(function (r) { _defineProperty(e, r, t[r]); }) : Object.getOwnPropertyDescriptors ? Object.defineProperties(e, Object.getOwnPropertyDescriptors(t)) : ownKeys(Object(t)).forEach(function (r) { Object.defineProperty(e, r, Object.getOwnPropertyDescriptor(t, r)); }); } return e; }
function _typeof(o) { "@babel/helpers - typeof"; return _typeof = "function" == typeof Symbol && "symbol" == typeof Symbol.iterator ? function (o) { return typeof o; } : function (o) { return o && "function" == typeof Symbol && o.constructor === Symbol && o !== Symbol.prototype ? "symbol" : typeof o; }, _typeof(o); }
function _classCallCheck(a, n) { if (!(a instanceof n)) throw new TypeError("Cannot call a class as a function"); }
function _defineProperties(e, r) { for (var t = 0; t < r.length; t++) { var o = r[t]; o.enumerable = o.enumerable || !1, o.configurable = !0, "value" in o && (o.writable = !0), Object.defineProperty(e, _toPropertyKey(o.key), o); } }
function _createClass(e, r, t) { return r && _defineProperties(e.prototype, r), t && _defineProperties(e, t), Object.defineProperty(e, "prototype", { writable: !1 }), e; }
function _defineProperty(e, r, t) { return (r = _toPropertyKey(r)) in e ? Object.defineProperty(e, r, { value: t, enumerable: !0, configurable: !0, writable: !0 }) : e[r] = t, e; }
function _toPropertyKey(t) { var i = _toPrimitive(t, "string"); return "symbol" == _typeof(i) ? i : i + ""; }
function _toPrimitive(t, r) { if ("object" != _typeof(t) || !t) return t; var e = t[Symbol.toPrimitive]; if (void 0 !== e) { var i = e.call(t, r || "default"); if ("object" != _typeof(i)) return i; throw new TypeError("@@toPrimitive must return a primitive value."); } return ("string" === r ? String : Number)(t); }
function _slicedToArray(r, e) { return _arrayWithHoles(r) || _iterableToArrayLimit(r, e) || _unsupportedIterableToArray(r, e) || _nonIterableRest(); }
function _nonIterableRest() { throw new TypeError("Invalid attempt to destructure non-iterable instance.\nIn order to be iterable, non-array objects must have a [Symbol.iterator]() method."); }
function _unsupportedIterableToArray(r, a) { if (r) { if ("string" == typeof r) return _arrayLikeToArray(r, a); var t = {}.toString.call(r).slice(8, -1); return "Object" === t && r.constructor && (t = r.constructor.name), "Map" === t || "Set" === t ? Array.from(r) : "Arguments" === t || /^(?:Ui|I)nt(?:8|16|32)(?:Clamped)?Array$/.test(t) ? _arrayLikeToArray(r, a) : void 0; } }
function _arrayLikeToArray(r, a) { (null == a || a > r.length) && (a = r.length); for (var e = 0, n = Array(a); e < a; e++) n[e] = r[e]; return n; }
function _iterableToArrayLimit(r, l) { var t = null == r ? null : "undefined" != typeof Symbol && r[Symbol.iterator] || r["@@iterator"]; if (null != t) { var e, n, i, u, a = [], f = !0, o = !1; try { if (i = (t = t.call(r)).next, 0 === l) { if (Object(t) !== t) return; f = !1; } else for (; !(f = (e = i.call(t)).done) && (a.push(e.value), a.length !== l); f = !0); } catch (r) { o = !0, n = r; } finally { try { if (!f && null != t.return && (u = t.return(), Object(u) !== u)) return; } finally { if (o) throw n; } } return a; } }
function _arrayWithHoles(r) { if (Array.isArray(r)) return r; }
function enumToJsonSchema(udt) {
  var description = udt.doc().toString();
  var cases = udt.cases();
  var oneOf = [];
  cases.forEach(function (aCase) {
    var title = aCase.name().toString();
    var desc = aCase.doc().toString();
    oneOf.push({
      description: desc,
      title: title,
      enum: [aCase.value()],
      type: "number"
    });
  });
  var res = {
    oneOf: oneOf
  };
  if (description.length > 0) {
    res.description = description;
  }
  return res;
}
function isNumeric(field) {
  return /^\d+$/.test(field.name().toString());
}
function readObj(args, input) {
  var inputName = input.name().toString();
  var entry = Object.entries(args).find(function (_ref) {
    var _ref2 = _slicedToArray(_ref, 1),
      name = _ref2[0];
    return name === inputName;
  });
  if (!entry) {
    throw new Error("Missing field ".concat(inputName));
  }
  return entry[1];
}
function findCase(name) {
  return function matches(entry) {
    switch (entry.switch().value) {
      case _stellarBase.xdr.ScSpecUdtUnionCaseV0Kind.scSpecUdtUnionCaseTupleV0().value:
        {
          var tuple = entry.tupleCase();
          return tuple.name().toString() === name;
        }
      case _stellarBase.xdr.ScSpecUdtUnionCaseV0Kind.scSpecUdtUnionCaseVoidV0().value:
        {
          var voidCase = entry.voidCase();
          return voidCase.name().toString() === name;
        }
      default:
        return false;
    }
  };
}
function stringToScVal(str, ty) {
  switch (ty.value) {
    case _stellarBase.xdr.ScSpecType.scSpecTypeString().value:
      return _stellarBase.xdr.ScVal.scvString(str);
    case _stellarBase.xdr.ScSpecType.scSpecTypeSymbol().value:
      return _stellarBase.xdr.ScVal.scvSymbol(str);
    case _stellarBase.xdr.ScSpecType.scSpecTypeAddress().value:
      {
        var addr = _stellarBase.Address.fromString(str);
        return _stellarBase.xdr.ScVal.scvAddress(addr.toScAddress());
      }
    case _stellarBase.xdr.ScSpecType.scSpecTypeU64().value:
      return new _stellarBase.XdrLargeInt("u64", str).toScVal();
    case _stellarBase.xdr.ScSpecType.scSpecTypeI64().value:
      return new _stellarBase.XdrLargeInt("i64", str).toScVal();
    case _stellarBase.xdr.ScSpecType.scSpecTypeU128().value:
      return new _stellarBase.XdrLargeInt("u128", str).toScVal();
    case _stellarBase.xdr.ScSpecType.scSpecTypeI128().value:
      return new _stellarBase.XdrLargeInt("i128", str).toScVal();
    case _stellarBase.xdr.ScSpecType.scSpecTypeU256().value:
      return new _stellarBase.XdrLargeInt("u256", str).toScVal();
    case _stellarBase.xdr.ScSpecType.scSpecTypeI256().value:
      return new _stellarBase.XdrLargeInt("i256", str).toScVal();
    case _stellarBase.xdr.ScSpecType.scSpecTypeBytes().value:
    case _stellarBase.xdr.ScSpecType.scSpecTypeBytesN().value:
      return _stellarBase.xdr.ScVal.scvBytes(Buffer.from(str, "base64"));
    default:
      throw new TypeError("invalid type ".concat(ty.name, " specified for string value"));
  }
}
var PRIMITIVE_DEFINITONS = {
  U32: {
    type: "integer",
    minimum: 0,
    maximum: 4294967295
  },
  I32: {
    type: "integer",
    minimum: -2147483648,
    maximum: 2147483647
  },
  U64: {
    type: "string",
    pattern: "^([1-9][0-9]*|0)$",
    minLength: 1,
    maxLength: 20
  },
  I64: {
    type: "string",
    pattern: "^(-?[1-9][0-9]*|0)$",
    minLength: 1,
    maxLength: 21
  },
  U128: {
    type: "string",
    pattern: "^([1-9][0-9]*|0)$",
    minLength: 1,
    maxLength: 39
  },
  I128: {
    type: "string",
    pattern: "^(-?[1-9][0-9]*|0)$",
    minLength: 1,
    maxLength: 40
  },
  U256: {
    type: "string",
    pattern: "^([1-9][0-9]*|0)$",
    minLength: 1,
    maxLength: 78
  },
  I256: {
    type: "string",
    pattern: "^(-?[1-9][0-9]*|0)$",
    minLength: 1,
    maxLength: 79
  },
  Address: {
    type: "string",
    format: "address",
    description: "Address can be a public key or contract id"
  },
  ScString: {
    type: "string",
    description: "ScString is a string"
  },
  ScSymbol: {
    type: "string",
    description: "ScString is a string"
  },
  DataUrl: {
    type: "string",
    pattern: "^(?:[A-Za-z0-9+\\/]{4})*(?:[A-Za-z0-9+\\/]{2}==|[A-Za-z0-9+\\/]{3}=)?$"
  }
};
function typeRef(typeDef) {
  var t = typeDef.switch();
  var value = t.value;
  var ref;
  switch (value) {
    case _stellarBase.xdr.ScSpecType.scSpecTypeVal().value:
      {
        ref = "Val";
        break;
      }
    case _stellarBase.xdr.ScSpecType.scSpecTypeBool().value:
      {
        return {
          type: "boolean"
        };
      }
    case _stellarBase.xdr.ScSpecType.scSpecTypeVoid().value:
      {
        return {
          type: "null"
        };
      }
    case _stellarBase.xdr.ScSpecType.scSpecTypeError().value:
      {
        ref = "Error";
        break;
      }
    case _stellarBase.xdr.ScSpecType.scSpecTypeU32().value:
      {
        ref = "U32";
        break;
      }
    case _stellarBase.xdr.ScSpecType.scSpecTypeI32().value:
      {
        ref = "I32";
        break;
      }
    case _stellarBase.xdr.ScSpecType.scSpecTypeU64().value:
      {
        ref = "U64";
        break;
      }
    case _stellarBase.xdr.ScSpecType.scSpecTypeI64().value:
      {
        ref = "I64";
        break;
      }
    case _stellarBase.xdr.ScSpecType.scSpecTypeTimepoint().value:
      {
        throw new Error("Timepoint type not supported");
        ref = "Timepoint";
        break;
      }
    case _stellarBase.xdr.ScSpecType.scSpecTypeDuration().value:
      {
        throw new Error("Duration not supported");
        ref = "Duration";
        break;
      }
    case _stellarBase.xdr.ScSpecType.scSpecTypeU128().value:
      {
        ref = "U128";
        break;
      }
    case _stellarBase.xdr.ScSpecType.scSpecTypeI128().value:
      {
        ref = "I128";
        break;
      }
    case _stellarBase.xdr.ScSpecType.scSpecTypeU256().value:
      {
        ref = "U256";
        break;
      }
    case _stellarBase.xdr.ScSpecType.scSpecTypeI256().value:
      {
        ref = "I256";
        break;
      }
    case _stellarBase.xdr.ScSpecType.scSpecTypeBytes().value:
      {
        ref = "DataUrl";
        break;
      }
    case _stellarBase.xdr.ScSpecType.scSpecTypeString().value:
      {
        ref = "ScString";
        break;
      }
    case _stellarBase.xdr.ScSpecType.scSpecTypeSymbol().value:
      {
        ref = "ScSymbol";
        break;
      }
    case _stellarBase.xdr.ScSpecType.scSpecTypeAddress().value:
      {
        ref = "Address";
        break;
      }
    case _stellarBase.xdr.ScSpecType.scSpecTypeOption().value:
      {
        var opt = typeDef.option();
        return typeRef(opt.valueType());
      }
    case _stellarBase.xdr.ScSpecType.scSpecTypeResult().value:
      {
        break;
      }
    case _stellarBase.xdr.ScSpecType.scSpecTypeVec().value:
      {
        var arr = typeDef.vec();
        var reference = typeRef(arr.elementType());
        return {
          type: "array",
          items: reference
        };
      }
    case _stellarBase.xdr.ScSpecType.scSpecTypeMap().value:
      {
        var map = typeDef.map();
        var items = [typeRef(map.keyType()), typeRef(map.valueType())];
        return {
          type: "array",
          items: {
            type: "array",
            items: items,
            minItems: 2,
            maxItems: 2
          }
        };
      }
    case _stellarBase.xdr.ScSpecType.scSpecTypeTuple().value:
      {
        var tuple = typeDef.tuple();
        var minItems = tuple.valueTypes().length;
        var maxItems = minItems;
        var _items = tuple.valueTypes().map(typeRef);
        return {
          type: "array",
          items: _items,
          minItems: minItems,
          maxItems: maxItems
        };
      }
    case _stellarBase.xdr.ScSpecType.scSpecTypeBytesN().value:
      {
        var _arr = typeDef.bytesN();
        return {
          $ref: "#/definitions/DataUrl",
          maxLength: _arr.n()
        };
      }
    case _stellarBase.xdr.ScSpecType.scSpecTypeUdt().value:
      {
        var udt = typeDef.udt();
        ref = udt.name().toString();
        break;
      }
  }
  return {
    $ref: "#/definitions/".concat(ref)
  };
}
function isRequired(typeDef) {
  return typeDef.switch().value !== _stellarBase.xdr.ScSpecType.scSpecTypeOption().value;
}
function argsAndRequired(input) {
  var properties = {};
  var required = [];
  input.forEach(function (arg) {
    var aType = arg.type();
    var name = arg.name().toString();
    properties[name] = typeRef(aType);
    if (isRequired(aType)) {
      required.push(name);
    }
  });
  var res = {
    properties: properties
  };
  if (required.length > 0) {
    res.required = required;
  }
  return res;
}
function structToJsonSchema(udt) {
  var fields = udt.fields();
  if (fields.some(isNumeric)) {
    if (!fields.every(isNumeric)) {
      throw new Error("mixed numeric and non-numeric field names are not allowed");
    }
    var items = fields.map(function (_, i) {
      return typeRef(fields[i].type());
    });
    return {
      type: "array",
      items: items,
      minItems: fields.length,
      maxItems: fields.length
    };
  }
  var description = udt.doc().toString();
  var _argsAndRequired = argsAndRequired(fields),
    properties = _argsAndRequired.properties,
    required = _argsAndRequired.required;
  properties.additionalProperties = false;
  return {
    description: description,
    properties: properties,
    required: required,
    type: "object"
  };
}
function functionToJsonSchema(func) {
  var _argsAndRequired2 = argsAndRequired(func.inputs()),
    properties = _argsAndRequired2.properties,
    required = _argsAndRequired2.required;
  var args = {
    additionalProperties: false,
    properties: properties,
    type: "object"
  };
  if ((required === null || required === void 0 ? void 0 : required.length) > 0) {
    args.required = required;
  }
  var input = {
    properties: {
      args: args
    }
  };
  var outputs = func.outputs();
  var output = outputs.length > 0 ? typeRef(outputs[0]) : typeRef(_stellarBase.xdr.ScSpecTypeDef.scSpecTypeVoid());
  var description = func.doc().toString();
  if (description.length > 0) {
    input.description = description;
  }
  input.additionalProperties = false;
  output.additionalProperties = false;
  return {
    input: input,
    output: output
  };
}
function unionToJsonSchema(udt) {
  var description = udt.doc().toString();
  var cases = udt.cases();
  var oneOf = [];
  cases.forEach(function (aCase) {
    switch (aCase.switch().value) {
      case _stellarBase.xdr.ScSpecUdtUnionCaseV0Kind.scSpecUdtUnionCaseVoidV0().value:
        {
          var c = aCase.voidCase();
          var title = c.name().toString();
          oneOf.push({
            type: "object",
            title: title,
            properties: {
              tag: title
            },
            additionalProperties: false,
            required: ["tag"]
          });
          break;
        }
      case _stellarBase.xdr.ScSpecUdtUnionCaseV0Kind.scSpecUdtUnionCaseTupleV0().value:
        {
          var _c = aCase.tupleCase();
          var _title = _c.name().toString();
          oneOf.push({
            type: "object",
            title: _title,
            properties: {
              tag: _title,
              values: {
                type: "array",
                items: _c.type().map(typeRef)
              }
            },
            required: ["tag", "values"],
            additionalProperties: false
          });
        }
    }
  });
  var res = {
    oneOf: oneOf
  };
  if (description.length > 0) {
    res.description = description;
  }
  return res;
}
var Spec = exports.Spec = function () {
  function Spec(entries) {
    _classCallCheck(this, Spec);
    _defineProperty(this, "entries", []);
    if (entries.length === 0) {
      throw new Error("Contract spec must have at least one entry");
    }
    var entry = entries[0];
    if (typeof entry === "string") {
      this.entries = entries.map(function (s) {
        return _stellarBase.xdr.ScSpecEntry.fromXDR(s, "base64");
      });
    } else {
      this.entries = entries;
    }
  }
  return _createClass(Spec, [{
    key: "funcs",
    value: function funcs() {
      return this.entries.filter(function (entry) {
        return entry.switch().value === _stellarBase.xdr.ScSpecEntryKind.scSpecEntryFunctionV0().value;
      }).map(function (entry) {
        return entry.functionV0();
      });
    }
  }, {
    key: "getFunc",
    value: function getFunc(name) {
      var entry = this.findEntry(name);
      if (entry.switch().value !== _stellarBase.xdr.ScSpecEntryKind.scSpecEntryFunctionV0().value) {
        throw new Error("".concat(name, " is not a function"));
      }
      return entry.functionV0();
    }
  }, {
    key: "funcArgsToScVals",
    value: function funcArgsToScVals(name, args) {
      var _this = this;
      var fn = this.getFunc(name);
      return fn.inputs().map(function (input) {
        return _this.nativeToScVal(readObj(args, input), input.type());
      });
    }
  }, {
    key: "funcResToNative",
    value: function funcResToNative(name, val_or_base64) {
      var val = typeof val_or_base64 === "string" ? _stellarBase.xdr.ScVal.fromXDR(val_or_base64, "base64") : val_or_base64;
      var func = this.getFunc(name);
      var outputs = func.outputs();
      if (outputs.length === 0) {
        var type = val.switch();
        if (type.value !== _stellarBase.xdr.ScValType.scvVoid().value) {
          throw new Error("Expected void, got ".concat(type.name));
        }
        return null;
      }
      if (outputs.length > 1) {
        throw new Error("Multiple outputs not supported");
      }
      var output = outputs[0];
      if (output.switch().value === _stellarBase.xdr.ScSpecType.scSpecTypeResult().value) {
        return new _rust_result.Ok(this.scValToNative(val, output.result().okType()));
      }
      return this.scValToNative(val, output);
    }
  }, {
    key: "findEntry",
    value: function findEntry(name) {
      var entry = this.entries.find(function (e) {
        return e.value().name().toString() === name;
      });
      if (!entry) {
        throw new Error("no such entry: ".concat(name));
      }
      return entry;
    }
  }, {
    key: "nativeToScVal",
    value: function nativeToScVal(val, ty) {
      var _this2 = this;
      var t = ty.switch();
      var value = t.value;
      if (t.value === _stellarBase.xdr.ScSpecType.scSpecTypeUdt().value) {
        var udt = ty.udt();
        return this.nativeToUdt(val, udt.name().toString());
      }
      if (value === _stellarBase.xdr.ScSpecType.scSpecTypeOption().value) {
        var opt = ty.option();
        if (val === undefined) {
          return _stellarBase.xdr.ScVal.scvVoid();
        }
        return this.nativeToScVal(val, opt.valueType());
      }
      switch (_typeof(val)) {
        case "object":
          {
            var _val$constructor$name, _val$constructor;
            if (val === null) {
              switch (value) {
                case _stellarBase.xdr.ScSpecType.scSpecTypeVoid().value:
                  return _stellarBase.xdr.ScVal.scvVoid();
                default:
                  throw new TypeError("Type ".concat(ty, " was not void, but value was null"));
              }
            }
            if (val instanceof _stellarBase.xdr.ScVal) {
              return val;
            }
            if (val instanceof _stellarBase.Address) {
              if (ty.switch().value !== _stellarBase.xdr.ScSpecType.scSpecTypeAddress().value) {
                throw new TypeError("Type ".concat(ty, " was not address, but value was Address"));
              }
              return val.toScVal();
            }
            if (val instanceof _stellarBase.Contract) {
              if (ty.switch().value !== _stellarBase.xdr.ScSpecType.scSpecTypeAddress().value) {
                throw new TypeError("Type ".concat(ty, " was not address, but value was Address"));
              }
              return val.address().toScVal();
            }
            if (val instanceof Uint8Array || Buffer.isBuffer(val)) {
              var copy = Uint8Array.from(val);
              switch (value) {
                case _stellarBase.xdr.ScSpecType.scSpecTypeBytesN().value:
                  {
                    var bytesN = ty.bytesN();
                    if (copy.length !== bytesN.n()) {
                      throw new TypeError("expected ".concat(bytesN.n(), " bytes, but got ").concat(copy.length));
                    }
                    return _stellarBase.xdr.ScVal.scvBytes(copy);
                  }
                case _stellarBase.xdr.ScSpecType.scSpecTypeBytes().value:
                  return _stellarBase.xdr.ScVal.scvBytes(copy);
                default:
                  throw new TypeError("invalid type (".concat(ty, ") specified for Bytes and BytesN"));
              }
            }
            if (Array.isArray(val)) {
              switch (value) {
                case _stellarBase.xdr.ScSpecType.scSpecTypeVec().value:
                  {
                    var vec = ty.vec();
                    var elementType = vec.elementType();
                    return _stellarBase.xdr.ScVal.scvVec(val.map(function (v) {
                      return _this2.nativeToScVal(v, elementType);
                    }));
                  }
                case _stellarBase.xdr.ScSpecType.scSpecTypeTuple().value:
                  {
                    var tup = ty.tuple();
                    var valTypes = tup.valueTypes();
                    if (val.length !== valTypes.length) {
                      throw new TypeError("Tuple expects ".concat(valTypes.length, " values, but ").concat(val.length, " were provided"));
                    }
                    return _stellarBase.xdr.ScVal.scvVec(val.map(function (v, i) {
                      return _this2.nativeToScVal(v, valTypes[i]);
                    }));
                  }
                case _stellarBase.xdr.ScSpecType.scSpecTypeMap().value:
                  {
                    var map = ty.map();
                    var keyType = map.keyType();
                    var valueType = map.valueType();
                    return _stellarBase.xdr.ScVal.scvMap(val.map(function (entry) {
                      var key = _this2.nativeToScVal(entry[0], keyType);
                      var mapVal = _this2.nativeToScVal(entry[1], valueType);
                      return new _stellarBase.xdr.ScMapEntry({
                        key: key,
                        val: mapVal
                      });
                    }));
                  }
                default:
                  throw new TypeError("Type ".concat(ty, " was not vec, but value was Array"));
              }
            }
            if (val.constructor === Map) {
              if (value !== _stellarBase.xdr.ScSpecType.scSpecTypeMap().value) {
                throw new TypeError("Type ".concat(ty, " was not map, but value was Map"));
              }
              var scMap = ty.map();
              var _map = val;
              var entries = [];
              var values = _map.entries();
              var res = values.next();
              while (!res.done) {
                var _res$value = _slicedToArray(res.value, 2),
                  k = _res$value[0],
                  v = _res$value[1];
                var key = this.nativeToScVal(k, scMap.keyType());
                var mapval = this.nativeToScVal(v, scMap.valueType());
                entries.push(new _stellarBase.xdr.ScMapEntry({
                  key: key,
                  val: mapval
                }));
                res = values.next();
              }
              return _stellarBase.xdr.ScVal.scvMap(entries);
            }
            if (((_val$constructor$name = (_val$constructor = val.constructor) === null || _val$constructor === void 0 ? void 0 : _val$constructor.name) !== null && _val$constructor$name !== void 0 ? _val$constructor$name : "") !== "Object") {
              var _val$constructor2;
              throw new TypeError("cannot interpret ".concat((_val$constructor2 = val.constructor) === null || _val$constructor2 === void 0 ? void 0 : _val$constructor2.name, " value as ScVal (").concat(JSON.stringify(val), ")"));
            }
            throw new TypeError("Received object ".concat(val, "  did not match the provided type ").concat(ty));
          }
        case "number":
        case "bigint":
          {
            switch (value) {
              case _stellarBase.xdr.ScSpecType.scSpecTypeU32().value:
                return _stellarBase.xdr.ScVal.scvU32(val);
              case _stellarBase.xdr.ScSpecType.scSpecTypeI32().value:
                return _stellarBase.xdr.ScVal.scvI32(val);
              case _stellarBase.xdr.ScSpecType.scSpecTypeU64().value:
              case _stellarBase.xdr.ScSpecType.scSpecTypeI64().value:
              case _stellarBase.xdr.ScSpecType.scSpecTypeU128().value:
              case _stellarBase.xdr.ScSpecType.scSpecTypeI128().value:
              case _stellarBase.xdr.ScSpecType.scSpecTypeU256().value:
              case _stellarBase.xdr.ScSpecType.scSpecTypeI256().value:
                {
                  var intType = t.name.substring(10).toLowerCase();
                  return new _stellarBase.XdrLargeInt(intType, val).toScVal();
                }
              default:
                throw new TypeError("invalid type (".concat(ty, ") specified for integer"));
            }
          }
        case "string":
          return stringToScVal(val, t);
        case "boolean":
          {
            if (value !== _stellarBase.xdr.ScSpecType.scSpecTypeBool().value) {
              throw TypeError("Type ".concat(ty, " was not bool, but value was bool"));
            }
            return _stellarBase.xdr.ScVal.scvBool(val);
          }
        case "undefined":
          {
            if (!ty) {
              return _stellarBase.xdr.ScVal.scvVoid();
            }
            switch (value) {
              case _stellarBase.xdr.ScSpecType.scSpecTypeVoid().value:
              case _stellarBase.xdr.ScSpecType.scSpecTypeOption().value:
                return _stellarBase.xdr.ScVal.scvVoid();
              default:
                throw new TypeError("Type ".concat(ty, " was not void, but value was undefined"));
            }
          }
        case "function":
          return this.nativeToScVal(val(), ty);
        default:
          throw new TypeError("failed to convert typeof ".concat(_typeof(val), " (").concat(val, ")"));
      }
    }
  }, {
    key: "nativeToUdt",
    value: function nativeToUdt(val, name) {
      var entry = this.findEntry(name);
      switch (entry.switch()) {
        case _stellarBase.xdr.ScSpecEntryKind.scSpecEntryUdtEnumV0():
          if (typeof val !== "number") {
            throw new TypeError("expected number for enum ".concat(name, ", but got ").concat(_typeof(val)));
          }
          return this.nativeToEnum(val, entry.udtEnumV0());
        case _stellarBase.xdr.ScSpecEntryKind.scSpecEntryUdtStructV0():
          return this.nativeToStruct(val, entry.udtStructV0());
        case _stellarBase.xdr.ScSpecEntryKind.scSpecEntryUdtUnionV0():
          return this.nativeToUnion(val, entry.udtUnionV0());
        default:
          throw new Error("failed to parse udt ".concat(name));
      }
    }
  }, {
    key: "nativeToUnion",
    value: function nativeToUnion(val, union_) {
      var _this3 = this;
      var entryName = val.tag;
      var caseFound = union_.cases().find(function (entry) {
        var caseN = entry.value().name().toString();
        return caseN === entryName;
      });
      if (!caseFound) {
        throw new TypeError("no such enum entry: ".concat(entryName, " in ").concat(union_));
      }
      var key = _stellarBase.xdr.ScVal.scvSymbol(entryName);
      switch (caseFound.switch()) {
        case _stellarBase.xdr.ScSpecUdtUnionCaseV0Kind.scSpecUdtUnionCaseVoidV0():
          {
            return _stellarBase.xdr.ScVal.scvVec([key]);
          }
        case _stellarBase.xdr.ScSpecUdtUnionCaseV0Kind.scSpecUdtUnionCaseTupleV0():
          {
            var types = caseFound.tupleCase().type();
            if (Array.isArray(val.values)) {
              if (val.values.length !== types.length) {
                throw new TypeError("union ".concat(union_, " expects ").concat(types.length, " values, but got ").concat(val.values.length));
              }
              var scvals = val.values.map(function (v, i) {
                return _this3.nativeToScVal(v, types[i]);
              });
              scvals.unshift(key);
              return _stellarBase.xdr.ScVal.scvVec(scvals);
            }
            throw new Error("failed to parse union case ".concat(caseFound, " with ").concat(val));
          }
        default:
          throw new Error("failed to parse union ".concat(union_, " with ").concat(val));
      }
    }
  }, {
    key: "nativeToStruct",
    value: function nativeToStruct(val, struct) {
      var _this4 = this;
      var fields = struct.fields();
      if (fields.some(isNumeric)) {
        if (!fields.every(isNumeric)) {
          throw new Error("mixed numeric and non-numeric field names are not allowed");
        }
        return _stellarBase.xdr.ScVal.scvVec(fields.map(function (_, i) {
          return _this4.nativeToScVal(val[i], fields[i].type());
        }));
      }
      return _stellarBase.xdr.ScVal.scvMap(fields.map(function (field) {
        var name = field.name().toString();
        return new _stellarBase.xdr.ScMapEntry({
          key: _this4.nativeToScVal(name, _stellarBase.xdr.ScSpecTypeDef.scSpecTypeSymbol()),
          val: _this4.nativeToScVal(val[name], field.type())
        });
      }));
    }
  }, {
    key: "nativeToEnum",
    value: function nativeToEnum(val, enum_) {
      if (enum_.cases().some(function (entry) {
        return entry.value() === val;
      })) {
        return _stellarBase.xdr.ScVal.scvU32(val);
      }
      throw new TypeError("no such enum entry: ".concat(val, " in ").concat(enum_));
    }
  }, {
    key: "scValStrToNative",
    value: function scValStrToNative(scv, typeDef) {
      return this.scValToNative(_stellarBase.xdr.ScVal.fromXDR(scv, "base64"), typeDef);
    }
  }, {
    key: "scValToNative",
    value: function scValToNative(scv, typeDef) {
      var _this5 = this;
      var t = typeDef.switch();
      var value = t.value;
      if (value === _stellarBase.xdr.ScSpecType.scSpecTypeUdt().value) {
        return this.scValUdtToNative(scv, typeDef.udt());
      }
      switch (scv.switch().value) {
        case _stellarBase.xdr.ScValType.scvVoid().value:
          return undefined;
        case _stellarBase.xdr.ScValType.scvU64().value:
        case _stellarBase.xdr.ScValType.scvI64().value:
        case _stellarBase.xdr.ScValType.scvU128().value:
        case _stellarBase.xdr.ScValType.scvI128().value:
        case _stellarBase.xdr.ScValType.scvU256().value:
        case _stellarBase.xdr.ScValType.scvI256().value:
          return (0, _stellarBase.scValToBigInt)(scv);
        case _stellarBase.xdr.ScValType.scvVec().value:
          {
            if (value === _stellarBase.xdr.ScSpecType.scSpecTypeVec().value) {
              var _scv$vec;
              var vec = typeDef.vec();
              return ((_scv$vec = scv.vec()) !== null && _scv$vec !== void 0 ? _scv$vec : []).map(function (elm) {
                return _this5.scValToNative(elm, vec.elementType());
              });
            }
            if (value === _stellarBase.xdr.ScSpecType.scSpecTypeTuple().value) {
              var _scv$vec2;
              var tuple = typeDef.tuple();
              var valTypes = tuple.valueTypes();
              return ((_scv$vec2 = scv.vec()) !== null && _scv$vec2 !== void 0 ? _scv$vec2 : []).map(function (elm, i) {
                return _this5.scValToNative(elm, valTypes[i]);
              });
            }
            throw new TypeError("Type ".concat(typeDef, " was not vec, but ").concat(scv, " is"));
          }
        case _stellarBase.xdr.ScValType.scvAddress().value:
          return _stellarBase.Address.fromScVal(scv).toString();
        case _stellarBase.xdr.ScValType.scvMap().value:
          {
            var _scv$map;
            var map = (_scv$map = scv.map()) !== null && _scv$map !== void 0 ? _scv$map : [];
            if (value === _stellarBase.xdr.ScSpecType.scSpecTypeMap().value) {
              var typed = typeDef.map();
              var keyType = typed.keyType();
              var valueType = typed.valueType();
              var res = map.map(function (entry) {
                return [_this5.scValToNative(entry.key(), keyType), _this5.scValToNative(entry.val(), valueType)];
              });
              return res;
            }
            throw new TypeError("ScSpecType ".concat(t.name, " was not map, but ").concat(JSON.stringify(scv, null, 2), " is"));
          }
        case _stellarBase.xdr.ScValType.scvBool().value:
        case _stellarBase.xdr.ScValType.scvU32().value:
        case _stellarBase.xdr.ScValType.scvI32().value:
        case _stellarBase.xdr.ScValType.scvBytes().value:
          return scv.value();
        case _stellarBase.xdr.ScValType.scvString().value:
        case _stellarBase.xdr.ScValType.scvSymbol().value:
          {
            var _scv$value;
            if (value !== _stellarBase.xdr.ScSpecType.scSpecTypeString().value && value !== _stellarBase.xdr.ScSpecType.scSpecTypeSymbol().value) {
              throw new Error("ScSpecType ".concat(t.name, " was not string or symbol, but ").concat(JSON.stringify(scv, null, 2), " is"));
            }
            return (_scv$value = scv.value()) === null || _scv$value === void 0 ? void 0 : _scv$value.toString();
          }
        case _stellarBase.xdr.ScValType.scvTimepoint().value:
        case _stellarBase.xdr.ScValType.scvDuration().value:
          return (0, _stellarBase.scValToBigInt)(_stellarBase.xdr.ScVal.scvU64(scv.u64()));
        default:
          throw new TypeError("failed to convert ".concat(JSON.stringify(scv, null, 2), " to native type from type ").concat(t.name));
      }
    }
  }, {
    key: "scValUdtToNative",
    value: function scValUdtToNative(scv, udt) {
      var entry = this.findEntry(udt.name().toString());
      switch (entry.switch()) {
        case _stellarBase.xdr.ScSpecEntryKind.scSpecEntryUdtEnumV0():
          return this.enumToNative(scv);
        case _stellarBase.xdr.ScSpecEntryKind.scSpecEntryUdtStructV0():
          return this.structToNative(scv, entry.udtStructV0());
        case _stellarBase.xdr.ScSpecEntryKind.scSpecEntryUdtUnionV0():
          return this.unionToNative(scv, entry.udtUnionV0());
        default:
          throw new Error("failed to parse udt ".concat(udt.name().toString(), ": ").concat(entry));
      }
    }
  }, {
    key: "unionToNative",
    value: function unionToNative(val, udt) {
      var _this6 = this;
      var vec = val.vec();
      if (!vec) {
        throw new Error("".concat(JSON.stringify(val, null, 2), " is not a vec"));
      }
      if (vec.length === 0 && udt.cases.length !== 0) {
        throw new Error("".concat(val, " has length 0, but the there are at least one case in the union"));
      }
      var name = vec[0].sym().toString();
      if (vec[0].switch().value !== _stellarBase.xdr.ScValType.scvSymbol().value) {
        throw new Error("{vec[0]} is not a symbol");
      }
      var entry = udt.cases().find(findCase(name));
      if (!entry) {
        throw new Error("failed to find entry ".concat(name, " in union {udt.name().toString()}"));
      }
      var res = {
        tag: name
      };
      if (entry.switch().value === _stellarBase.xdr.ScSpecUdtUnionCaseV0Kind.scSpecUdtUnionCaseTupleV0().value) {
        var tuple = entry.tupleCase();
        var ty = tuple.type();
        var values = ty.map(function (e, i) {
          return _this6.scValToNative(vec[i + 1], e);
        });
        res.values = values;
      }
      return res;
    }
  }, {
    key: "structToNative",
    value: function structToNative(val, udt) {
      var _this7 = this,
        _val$map;
      var res = {};
      var fields = udt.fields();
      if (fields.some(isNumeric)) {
        var _val$vec;
        var r = (_val$vec = val.vec()) === null || _val$vec === void 0 ? void 0 : _val$vec.map(function (entry, i) {
          return _this7.scValToNative(entry, fields[i].type());
        });
        return r;
      }
      (_val$map = val.map()) === null || _val$map === void 0 || _val$map.forEach(function (entry, i) {
        var field = fields[i];
        res[field.name().toString()] = _this7.scValToNative(entry.val(), field.type());
      });
      return res;
    }
  }, {
    key: "enumToNative",
    value: function enumToNative(scv) {
      if (scv.switch().value !== _stellarBase.xdr.ScValType.scvU32().value) {
        throw new Error("Enum must have a u32 value");
      }
      var num = scv.u32();
      return num;
    }
  }, {
    key: "errorCases",
    value: function errorCases() {
      return this.entries.filter(function (entry) {
        return entry.switch().value === _stellarBase.xdr.ScSpecEntryKind.scSpecEntryUdtErrorEnumV0().value;
      }).flatMap(function (entry) {
        return entry.value().cases();
      });
    }
  }, {
    key: "jsonSchema",
    value: function jsonSchema(funcName) {
      var definitions = {};
      this.entries.forEach(function (entry) {
        switch (entry.switch().value) {
          case _stellarBase.xdr.ScSpecEntryKind.scSpecEntryUdtEnumV0().value:
            {
              var udt = entry.udtEnumV0();
              definitions[udt.name().toString()] = enumToJsonSchema(udt);
              break;
            }
          case _stellarBase.xdr.ScSpecEntryKind.scSpecEntryUdtStructV0().value:
            {
              var _udt = entry.udtStructV0();
              definitions[_udt.name().toString()] = structToJsonSchema(_udt);
              break;
            }
          case _stellarBase.xdr.ScSpecEntryKind.scSpecEntryUdtUnionV0().value:
            {
              var _udt2 = entry.udtUnionV0();
              definitions[_udt2.name().toString()] = unionToJsonSchema(_udt2);
              break;
            }
          case _stellarBase.xdr.ScSpecEntryKind.scSpecEntryFunctionV0().value:
            {
              var fn = entry.functionV0();
              var fnName = fn.name().toString();
              var _functionToJsonSchema = functionToJsonSchema(fn),
                input = _functionToJsonSchema.input;
              definitions[fnName] = input;
              break;
            }
          case _stellarBase.xdr.ScSpecEntryKind.scSpecEntryUdtErrorEnumV0().value:
            {}
        }
      });
      var res = {
        $schema: "http://json-schema.org/draft-07/schema#",
        definitions: _objectSpread(_objectSpread({}, PRIMITIVE_DEFINITONS), definitions)
      };
      if (funcName) {
        res.$ref = "#/definitions/".concat(funcName);
      }
      return res;
    }
  }]);
}();