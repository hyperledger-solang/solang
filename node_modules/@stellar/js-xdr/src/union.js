import { Void } from './void';
import { Reference } from './reference';
import { XdrCompositeType, isSerializableIsh } from './xdr-type';
import { XdrWriterError } from './errors';

export class Union extends XdrCompositeType {
  constructor(aSwitch, value) {
    super();
    this.set(aSwitch, value);
  }

  set(aSwitch, value) {
    if (typeof aSwitch === 'string') {
      aSwitch = this.constructor._switchOn.fromName(aSwitch);
    }

    this._switch = aSwitch;
    const arm = this.constructor.armForSwitch(this._switch);
    this._arm = arm;
    this._armType = arm === Void ? Void : this.constructor._arms[arm];
    this._value = value;
  }

  get(armName = this._arm) {
    if (this._arm !== Void && this._arm !== armName)
      throw new TypeError(`${armName} not set`);
    return this._value;
  }

  switch() {
    return this._switch;
  }

  arm() {
    return this._arm;
  }

  armType() {
    return this._armType;
  }

  value() {
    return this._value;
  }

  static armForSwitch(aSwitch) {
    const member = this._switches.get(aSwitch);
    if (member !== undefined) {
      return member;
    }
    if (this._defaultArm) {
      return this._defaultArm;
    }
    throw new TypeError(`Bad union switch: ${aSwitch}`);
  }

  static armTypeForArm(arm) {
    if (arm === Void) {
      return Void;
    }
    return this._arms[arm];
  }

  /**
   * @inheritDoc
   */
  static read(reader) {
    const aSwitch = this._switchOn.read(reader);
    const arm = this.armForSwitch(aSwitch);
    const armType = arm === Void ? Void : this._arms[arm];
    let value;
    if (armType !== undefined) {
      value = armType.read(reader);
    } else {
      value = arm.read(reader);
    }
    return new this(aSwitch, value);
  }

  /**
   * @inheritDoc
   */
  static write(value, writer) {
    if (!this.isValid(value)) {
      throw new XdrWriterError(
        `${value} has union name ${value?.unionName}, not ${
          this.unionName
        }: ${JSON.stringify(value)}`
      );
    }

    this._switchOn.write(value.switch(), writer);
    value.armType().write(value.value(), writer);
  }

  /**
   * @inheritDoc
   */
  static isValid(value) {
    return (
      value?.constructor?.unionName === this.unionName ||
      isSerializableIsh(value, this)
    );
  }

  static create(context, name, config) {
    const ChildUnion = class extends Union {};

    ChildUnion.unionName = name;
    context.results[name] = ChildUnion;

    if (config.switchOn instanceof Reference) {
      ChildUnion._switchOn = config.switchOn.resolve(context);
    } else {
      ChildUnion._switchOn = config.switchOn;
    }

    ChildUnion._switches = new Map();
    ChildUnion._arms = {};

    // resolve default arm
    let defaultArm = config.defaultArm;
    if (defaultArm instanceof Reference) {
      defaultArm = defaultArm.resolve(context);
    }

    ChildUnion._defaultArm = defaultArm;

    for (const [aSwitch, armName] of config.switches) {
      const key =
        typeof aSwitch === 'string'
          ? ChildUnion._switchOn.fromName(aSwitch)
          : aSwitch;

      ChildUnion._switches.set(key, armName);
    }

    // add enum-based helpers
    // NOTE: we don't have good notation for "is a subclass of XDR.Enum",
    //  and so we use the following check (does _switchOn have a `values`
    //  attribute) to approximate the intent.
    if (ChildUnion._switchOn.values !== undefined) {
      for (const aSwitch of ChildUnion._switchOn.values()) {
        // Add enum-based constructors
        ChildUnion[aSwitch.name] = function ctr(value) {
          return new ChildUnion(aSwitch, value);
        };

        // Add enum-based "set" helpers
        ChildUnion.prototype[aSwitch.name] = function set(value) {
          return this.set(aSwitch, value);
        };
      }
    }

    if (config.arms) {
      for (const [armsName, value] of Object.entries(config.arms)) {
        ChildUnion._arms[armsName] =
          value instanceof Reference ? value.resolve(context) : value;
        // Add arm accessor helpers
        if (value !== Void) {
          ChildUnion.prototype[armsName] = function get() {
            return this.get(armsName);
          };
        }
      }
    }

    return ChildUnion;
  }
}
