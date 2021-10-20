import { snakeCase } from "snake-case";
import { sha256 } from "js-sha256";
import { Idl, IdlField, IdlTypeDef, IdlEnumVariant, IdlType } from "../idl";
import { IdlError } from "../error";

export function accountSize(idl: Idl, idlAccount: IdlTypeDef): number {
  if (idlAccount.type.kind === "enum") {
    let variantSizes = idlAccount.type.variants.map(
      (variant: IdlEnumVariant) => {
        if (variant.fields === undefined) {
          return 0;
        }
        return variant.fields
          .map((f: IdlField | IdlType) => {
            if (!(typeof f === "object" && "name" in f)) {
              throw new Error("Tuple enum variants not yet implemented.");
            }
            return typeSize(idl, f.type);
          })
          .reduce((a: number, b: number) => a + b);
      }
    );
    return Math.max(...variantSizes) + 1;
  }
  if (idlAccount.type.fields === undefined) {
    return 0;
  }
  return idlAccount.type.fields
    .map((f) => typeSize(idl, f.type))
    .reduce((a, b) => a + b, 0);
}

// Returns the size of the type in bytes. For variable length types, just return
// 1. Users should override this value in such cases.
function typeSize(idl: Idl, ty: IdlType): number {
  switch (ty) {
    case "Bool":
      return 1;
    case "U8":
      return 1;
    case "I8":
      return 1;
    case "I16":
      return 2;
    case "U16":
      return 2;
    case "U32":
      return 4;
    case "I32":
      return 4;
    case "U64":
      return 8;
    case "I64":
      return 8;
    case "U128":
      return 16;
    case "I128":
      return 16;
    case "Bytes":
      return 1;
    case "String":
      return 1;
    case "PublicKey":
      return 32;
    default:
      if ("Vec" in ty) {
        return 1;
      }
      if ("Option" in ty) {
        return 1 + typeSize(idl, ty.Option);
      }
      if ("Defined" in ty) {
        const filtered = idl.types?.filter((t) => t.name === ty.Defined) ?? [];
        if (filtered.length !== 1) {
          throw new IdlError(`Type not found: ${JSON.stringify(ty)}`);
        }
        let typeDef = filtered[0];

        return accountSize(idl, typeDef);
      }
      if ("Array" in ty) {
        let arrayTy = ty.Array[0];
        let arraySize = ty.Array[1];
        return typeSize(idl, arrayTy) * arraySize;
      }
      throw new Error(`Invalid type ${JSON.stringify(ty)}`);
  }
}

// Not technically sighash, since we don't include the arguments, as Rust
// doesn't allow function overloading.
export function sighash(nameSpace: string, ixName: string): Buffer {
  let name = snakeCase(ixName);
  let preimage = `${nameSpace}:${name}`;
  return Buffer.from(sha256.digest(preimage)).slice(0, 8);
}
