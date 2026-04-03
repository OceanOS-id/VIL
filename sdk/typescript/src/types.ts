/**
 * VIL field type helpers — used in schema declarations.
 */

export interface FieldSpec {
  type: string;
  required?: boolean;
  items_type?: string;
}

export interface UpstreamSpec {
  type: string;
  url: string;
  method?: string;
  body_template?: string;
}

/** Declare a String field. */
export function string(required = false): FieldSpec {
  return { type: 'String', required };
}

/** Declare a u64 field. */
export function number(required = false): FieldSpec {
  return { type: 'u64', required };
}

/** Declare a bool field. */
export function boolean(required = false): FieldSpec {
  return { type: 'bool', required };
}

/** Declare a Vec<T> field. */
export function array(items = 'string'): FieldSpec {
  return { type: `Vec<${items}>`, required: false };
}

/** Declare an SSE upstream connection. */
export function sse(url: string, body?: string): UpstreamSpec {
  const result: UpstreamSpec = { type: 'sse', url };
  if (body) {
    result.body_template = body;
  }
  return result;
}

/** Declare an HTTP upstream connection. */
export function http(url: string, method = 'POST', body?: string): UpstreamSpec {
  const result: UpstreamSpec = { type: 'http', url, method };
  if (body) {
    result.body_template = body;
  }
  return result;
}
