// Minimal JSON Schema (draft-07 subset) validator for benchmark contracts.
// Supports: type, properties, required, additionalProperties:false, enum,
// items, minItems, minLength, minimum, pattern, $defs/$ref (#/... local),
// allOf. $comment keys are ignored. Deliberately small; anything beyond this
// subset should move to a real validator library.

function resolveRef(rootSchema, ref) {
  if (!ref.startsWith("#/")) throw new Error(`unsupported $ref ${ref}`);
  let node = rootSchema;
  for (const segment of ref.slice(2).split("/")) {
    node = node?.[segment];
    if (node === undefined) throw new Error(`unresolvable $ref ${ref}`);
  }
  return node;
}

function checkType(value, type) {
  switch (type) {
    case "object":
      return typeof value === "object" && value !== null && !Array.isArray(value);
    case "array":
      return Array.isArray(value);
    case "string":
      return typeof value === "string";
    case "integer":
      return Number.isInteger(value);
    case "number":
      return typeof value === "number" && Number.isFinite(value);
    case "boolean":
      return typeof value === "boolean";
    default:
      throw new Error(`unsupported schema type ${type}`);
  }
}

export function validateAgainstSchema(schema, value, rootSchema = schema, label = "value") {
  const errors = [];
  const visit = (node, current, path) => {
    if (!node || typeof node !== "object") {
      errors.push(`${path}: invalid schema node`);
      return;
    }
    if (typeof node.$ref === "string") {
      visit(resolveRef(rootSchema, node.$ref), current, path);
      return;
    }
    if (Array.isArray(node.allOf)) {
      for (const sub of node.allOf) visit(sub, current, path);
    }
    if (node.type && !checkType(current, node.type)) {
      errors.push(`${path}: expected ${node.type}`);
      return;
    }
    if (Array.isArray(node.enum) && !node.enum.includes(current)) {
      errors.push(`${path}: must be one of ${JSON.stringify(node.enum)}`);
    }
    if (typeof node.minLength === "number" && typeof current === "string" && current.length < node.minLength) {
      errors.push(`${path}: shorter than minLength ${node.minLength}`);
    }
    if (typeof node.pattern === "string" && typeof current === "string" && !new RegExp(node.pattern).test(current)) {
      errors.push(`${path}: does not match pattern ${node.pattern}`);
    }
    if (typeof node.minimum === "number" && typeof current === "number" && current < node.minimum) {
      errors.push(`${path}: below minimum ${node.minimum}`);
    }
    if (node.type === "object" && typeof current === "object" && current !== null) {
      for (const key of node.required ?? []) {
        if (!(key in current)) errors.push(`${path}: missing required property '${key}'`);
      }
      for (const [key, child] of Object.entries(node.properties ?? {})) {
        if (key in current) visit(child, current[key], `${path}.${key}`);
      }
      if (node.additionalProperties === false) {
        const allowed = new Set(Object.keys(node.properties ?? {}));
        for (const key of Object.keys(current)) {
          if (!allowed.has(key)) errors.push(`${path}: unexpected property '${key}'`);
        }
      }
    }
    if (node.type === "array" && Array.isArray(current)) {
      if (typeof node.minItems === "number" && current.length < node.minItems) {
        errors.push(`${path}: fewer than ${node.minItems} items`);
      }
      if (node.items) {
        for (const [index, item] of current.entries()) visit(node.items, item, `${path}[${index}]`);
      }
    }
  };
  visit(schema, value, label);
  return { ok: errors.length === 0, errors };
}
