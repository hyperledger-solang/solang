const HASH_SIZE = 32;

typedef opaque Signature[32];

enum ResultType 
{
  OK    = 0,
  ERROR = 1
};

union Result switch(ResultType type)
{    
  case OK:
    void;
  case ERROR:
    int code;
};

typedef unsigned int ColorCode;

struct Color
{
  ColorCode red;
  ColorCode green;
  ColorCode blue;
};


struct Exhaustive 
{

  // Bools
  bool   aBool;
  bool*  anOptionalBool;
  bool   aBoolArray[5];
  bool   aBoolVarArray<5>;
  bool   anUnboundedBoolVarArray<>;

  // Ints
  int   anInt;
  int*  anOptionalInt;
  int   anIntArray[5];
  int   anIntVarArray<5>;
  int   anUnboundedIntVarArray<>;

  // Uints
  unsigned int   anUnsignedInt;
  unsigned int*  anOptionalUnsignedInt;
  unsigned int   anUnsignedIntArray[5];
  unsigned int   anUnsignedIntVarArray<5>;
  unsigned int   anUnboundedUnsignedIntVarArray<>;

  // Hypers
  hyper   aHyper;
  hyper*  anOptionalHyper;
  hyper   aHyperArray[5];
  hyper   aHyperVarArray<5>;
  hyper   anUnboundedHyperVarArray<>;

  // Uhypers
  unsigned hyper   anUnsignedHyper;
  unsigned hyper*  anOptionalUnsignedHyper;
  unsigned hyper   anUnsignedHyperArray[5];
  unsigned hyper   anUnsignedHyperVarArray<5>;
  unsigned hyper   anUnboundedUnsignedHyperVarArray<>;

  // Floats
  float   aFloat;
  float*  anOptionalFloat;
  float   aFloatArray[5];
  float   aFloatVarArray<5>;
  float   anUnboundedFloatVarArray<>;


  // Doubles
  double   aDouble;
  double*  anOptionalDouble;
  double   aDoubleArray[5];
  double   aDoubleVarArray<5>;
  double   anUnboundedDoubleVarArray<>;


  // Opaque
  opaque   anOpaque[10];

  // VarOpaque
  opaque   aVarOpaque<10>;
  opaque   anUnboundedVarOpaque<>;

  // String
  string   aString<19>;
  string   anUnboundedString<>;


  // Typedef
  Signature   aSignature;
  Signature*  anOptionalSignature;
  Signature   aSignatureArray[5];
  Signature   aSignatureVarArray<5>;
  Signature   anUnboundedSignatureVarArray<>;

  // Enum
  ResultType   aResultType;
  ResultType*  anOptionalResultType;
  ResultType   aResultTypeArray[5];
  ResultType   aResultTypeVarArray<5>;
  ResultType   anUnboundedResultTypeVarArray<>;


  // Struct
  Color   aColor;
  Color*  anOptionalColor;
  Color   aColorArray[5];
  Color   aColorVarArray<5>;
  Color   anUnboundedColorVarArray<>;

  // Union
  Result   aResult;
  Result*  anOptionalResult;
  Result   aResultArray[5];
  Result   aResultVarArray<5>;
  Result   anUnboundedResultVarArray<>;

  //Nested enum
  enum { OK = 0 } aNestedEnum;
  enum { OK = 0 } *anOptionalNestedEnum;
  enum { OK = 0 } aNestedEnumArray[3];
  enum { OK = 0 } aNestedEnumVarArray<3>;
  enum { OK = 0 } anUnboundedNestedEnumVarArray<>;
  
  //Nested Struct
  struct { int  value; } aNestedStruct;
  struct { int  value; } *anOptionalNestedStruct;
  struct { int  value; } aNestedStructArray[3];
  struct { int  value; } aNestedStructVarArray<3>;
  struct { int  value; } anUnboundedNestedStructVarArray<>;

};

enum ExhaustiveUnionType {
  VOID_ARM               = 0,

  PRIMITIVE_SIMPLE_ARM   = 1,
  PRIMITIVE_OPTIONAL_ARM = 2,
  PRIMITIVE_ARRAY_ARM    = 2,
  PRIMITIVE_VARARRAY_ARM = 3,

  CUSTOM_SIMPLE_ARM      = 4,
  CUSTOM_OPTIONAL_ARM    = 5,
  CUSTOM_ARRAY_ARM       = 6,
  CUSTOM_VARARRAY_ARM    = 7,

  FOR_DEFAULT            = -1
};


union ExhaustiveUnion switch(ExhaustiveUnionType type)
{
  case VOID_ARM:               void;
  case PRIMITIVE_SIMPLE_ARM:   int aPrimitiveSimpleArm;
  case PRIMITIVE_OPTIONAL_ARM: int* aPrimitiveOptionalArm;

  default: int aPrimitiveDefault;
};