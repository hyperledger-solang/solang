// eslint-disable-next-line max-classes-per-file
import * as XDRTypes from './types';
import { Reference } from './reference';
import { XdrDefinitionError } from './errors';

export * from './reference';

class SimpleReference extends Reference {
  constructor(name) {
    super();
    this.name = name;
  }

  resolve(context) {
    const defn = context.definitions[this.name];
    return defn.resolve(context);
  }
}

class ArrayReference extends Reference {
  constructor(childReference, length, variable = false) {
    super();
    this.childReference = childReference;
    this.length = length;
    this.variable = variable;
  }

  resolve(context) {
    let resolvedChild = this.childReference;
    let length = this.length;

    if (resolvedChild instanceof Reference) {
      resolvedChild = resolvedChild.resolve(context);
    }

    if (length instanceof Reference) {
      length = length.resolve(context);
    }

    if (this.variable) {
      return new XDRTypes.VarArray(resolvedChild, length);
    }
    return new XDRTypes.Array(resolvedChild, length);
  }
}

class OptionReference extends Reference {
  constructor(childReference) {
    super();
    this.childReference = childReference;
    this.name = childReference.name;
  }

  resolve(context) {
    let resolvedChild = this.childReference;

    if (resolvedChild instanceof Reference) {
      resolvedChild = resolvedChild.resolve(context);
    }

    return new XDRTypes.Option(resolvedChild);
  }
}

class SizedReference extends Reference {
  constructor(sizedType, length) {
    super();
    this.sizedType = sizedType;
    this.length = length;
  }

  resolve(context) {
    let length = this.length;

    if (length instanceof Reference) {
      length = length.resolve(context);
    }

    return new this.sizedType(length);
  }
}

class Definition {
  constructor(constructor, name, cfg) {
    this.constructor = constructor;
    this.name = name;
    this.config = cfg;
  }

  // resolve calls the constructor of this definition with the provided context
  // and this definitions config values.  The definitions constructor should
  // populate the final type on `context.results`, and may refer to other
  // definitions through `context.definitions`
  resolve(context) {
    if (this.name in context.results) {
      return context.results[this.name];
    }

    return this.constructor(context, this.name, this.config);
  }
}

// let the reference resolution system do its thing
// the "constructor" for a typedef just returns the resolved value
function createTypedef(context, typeName, value) {
  if (value instanceof Reference) {
    value = value.resolve(context);
  }
  context.results[typeName] = value;
  return value;
}

function createConst(context, name, value) {
  context.results[name] = value;
  return value;
}

class TypeBuilder {
  constructor(destination) {
    this._destination = destination;
    this._definitions = {};
  }

  enum(name, members) {
    const result = new Definition(XDRTypes.Enum.create, name, members);
    this.define(name, result);
  }

  struct(name, members) {
    const result = new Definition(XDRTypes.Struct.create, name, members);
    this.define(name, result);
  }

  union(name, cfg) {
    const result = new Definition(XDRTypes.Union.create, name, cfg);
    this.define(name, result);
  }

  typedef(name, cfg) {
    const result = new Definition(createTypedef, name, cfg);
    this.define(name, result);
  }

  const(name, cfg) {
    const result = new Definition(createConst, name, cfg);
    this.define(name, result);
  }

  void() {
    return XDRTypes.Void;
  }

  bool() {
    return XDRTypes.Bool;
  }

  int() {
    return XDRTypes.Int;
  }

  hyper() {
    return XDRTypes.Hyper;
  }

  uint() {
    return XDRTypes.UnsignedInt;
  }

  uhyper() {
    return XDRTypes.UnsignedHyper;
  }

  float() {
    return XDRTypes.Float;
  }

  double() {
    return XDRTypes.Double;
  }

  quadruple() {
    return XDRTypes.Quadruple;
  }

  string(length) {
    return new SizedReference(XDRTypes.String, length);
  }

  opaque(length) {
    return new SizedReference(XDRTypes.Opaque, length);
  }

  varOpaque(length) {
    return new SizedReference(XDRTypes.VarOpaque, length);
  }

  array(childType, length) {
    return new ArrayReference(childType, length);
  }

  varArray(childType, maxLength) {
    return new ArrayReference(childType, maxLength, true);
  }

  option(childType) {
    return new OptionReference(childType);
  }

  define(name, definition) {
    if (this._destination[name] === undefined) {
      this._definitions[name] = definition;
    } else {
      throw new XdrDefinitionError(`${name} is already defined`);
    }
  }

  lookup(name) {
    return new SimpleReference(name);
  }

  resolve() {
    for (const defn of Object.values(this._definitions)) {
      defn.resolve({
        definitions: this._definitions,
        results: this._destination
      });
    }
  }
}

export function config(fn, types = {}) {
  if (fn) {
    const builder = new TypeBuilder(types);
    fn(builder);
    builder.resolve();
  }

  return types;
}
