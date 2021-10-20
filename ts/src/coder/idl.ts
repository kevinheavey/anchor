import camelCase from "camelcase";
import { Layout } from "buffer-layout";
import * as borsh from "@project-serum/borsh";
import { IdlField, IdlTypeDef, IdlEnumVariant, IdlType } from "../idl";
import { IdlError } from "../error";

export class IdlCoder {
  public static fieldLayout(
    field: { name?: string } & Pick<IdlField, "type">,
    types?: IdlTypeDef[]
  ): Layout {
    const fieldName =
      field.name !== undefined ? camelCase(field.name) : undefined;
    switch (field.type) {
      case "Bool": {
        return borsh.bool(fieldName);
      }
      case "U8": {
        return borsh.u8(fieldName);
      }
      case "I8": {
        return borsh.i8(fieldName);
      }
      case "U16": {
        return borsh.u16(fieldName);
      }
      case "I16": {
        return borsh.i16(fieldName);
      }
      case "U32": {
        return borsh.u32(fieldName);
      }
      case "I32": {
        return borsh.i32(fieldName);
      }
      case "U64": {
        return borsh.u64(fieldName);
      }
      case "I64": {
        return borsh.i64(fieldName);
      }
      case "U128": {
        return borsh.u128(fieldName);
      }
      case "I128": {
        return borsh.i128(fieldName);
      }
      case "Bytes": {
        return borsh.vecU8(fieldName);
      }
      case "String": {
        return borsh.str(fieldName);
      }
      case "PublicKey": {
        return borsh.publicKey(fieldName);
      }
      default: {
        if ("Vec" in field.type) {
          return borsh.vec(
            IdlCoder.fieldLayout(
              {
                name: undefined,
                // @ts-ignore
                type: field.type.vec,
              },
              types
            ),
            fieldName
          );
        } else if ("Option" in field.type) {
          return borsh.option(
            IdlCoder.fieldLayout(
              {
                name: undefined,
                type: field.type.Option,
              },
              types
            ),
            fieldName
          );
        } else if ("Defined" in field.type) {
          const defined = field.type.Defined;
          // User defined type.
          if (types === undefined) {
            throw new IdlError("User defined types not provided");
          }
          const filtered = types.filter((t) => t.name === defined);
          if (filtered.length !== 1) {
            throw new IdlError(`Type not found: ${JSON.stringify(field)}`);
          }
          return IdlCoder.typeDefLayout(filtered[0], types, fieldName);
        } else if ("Array" in field.type) {
          let arrayTy = field.type.Array[0];
          let arrayLen = field.type.Array[1];
          let innerLayout = IdlCoder.fieldLayout(
            {
              name: undefined,
              type: arrayTy,
            },
            types
          );
          return borsh.array(innerLayout, arrayLen, fieldName);
        } else {
          throw new Error(`Not yet implemented: ${field}`);
        }
      }
    }
  }

  public static typeDefLayout(
    typeDef: IdlTypeDef,
    types: IdlTypeDef[] = [],
    name?: string
  ): Layout {
    if (typeDef.type.kind === "struct") {
      const fieldLayouts = typeDef.type.fields.map((field) => {
        const x = IdlCoder.fieldLayout(field, types);
        return x;
      });
      return borsh.struct(fieldLayouts, name);
    } else if (typeDef.type.kind === "enum") {
      let variants = typeDef.type.variants.map((variant: IdlEnumVariant) => {
        const name = camelCase(variant.name);
        if (variant.fields === undefined) {
          return borsh.struct([], name);
        }
        // @ts-ignore
        const fieldLayouts = variant.fields.map((f: IdlField | IdlType) => {
          // @ts-ignore
          if (f.name === undefined) {
            throw new Error("Tuple enum variants not yet implemented.");
          }
          // @ts-ignore
          return IdlCoder.fieldLayout(f, types);
        });
        return borsh.struct(fieldLayouts, name);
      });

      if (name !== undefined) {
        // Buffer-layout lib requires the name to be null (on construction)
        // when used as a field.
        return borsh.rustEnum(variants).replicate(name);
      }

      return borsh.rustEnum(variants, name);
    } else {
      throw new Error(`Unknown type kint: ${typeDef}`);
    }
  }
}
