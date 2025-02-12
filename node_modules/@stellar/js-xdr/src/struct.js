import { Reference } from './reference';
import { XdrCompositeType, isSerializableIsh } from './xdr-type';
import { XdrWriterError } from './errors';

export class Struct extends XdrCompositeType {
  constructor(attributes) {
    super();
    this._attributes = attributes || {};
  }

  /**
   * @inheritDoc
   */
  static read(reader) {
    const attributes = {};
    for (const [fieldName, type] of this._fields) {
      attributes[fieldName] = type.read(reader);
    }
    return new this(attributes);
  }

  /**
   * @inheritDoc
   */
  static write(value, writer) {
    if (!this.isValid(value)) {
      throw new XdrWriterError(
        `${value} has struct name ${value?.constructor?.structName}, not ${
          this.structName
        }: ${JSON.stringify(value)}`
      );
    }

    for (const [fieldName, type] of this._fields) {
      const attribute = value._attributes[fieldName];
      type.write(attribute, writer);
    }
  }

  /**
   * @inheritDoc
   */
  static isValid(value) {
    return (
      value?.constructor?.structName === this.structName ||
      isSerializableIsh(value, this)
    );
  }

  static create(context, name, fields) {
    const ChildStruct = class extends Struct {};

    ChildStruct.structName = name;

    context.results[name] = ChildStruct;

    const mappedFields = new Array(fields.length);
    for (let i = 0; i < fields.length; i++) {
      const fieldDescriptor = fields[i];
      const fieldName = fieldDescriptor[0];
      let field = fieldDescriptor[1];
      if (field instanceof Reference) {
        field = field.resolve(context);
      }
      mappedFields[i] = [fieldName, field];
      // create accessors
      ChildStruct.prototype[fieldName] = createAccessorMethod(fieldName);
    }

    ChildStruct._fields = mappedFields;

    return ChildStruct;
  }
}

function createAccessorMethod(name) {
  return function readOrWriteAttribute(value) {
    if (value !== undefined) {
      this._attributes[name] = value;
    }
    return this._attributes[name];
  };
}
