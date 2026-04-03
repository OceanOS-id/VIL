/**
 * Internal YAML generation helpers — no external YAML deps.
 * String building approach matching the Python SDK output format.
 */

export interface FieldEntry {
  name: string;
  type: string;
  required?: boolean;
  items_type?: string;
}

export interface SemanticEntry {
  name: string;
  kind: string;
  fields: FieldEntry[];
  variants: string[];
}

export interface ErrorEntry {
  name: string;
  status: number;
  code?: string;
  retry?: boolean;
  fields: FieldEntry[];
}

export interface StateEntry {
  type: string;
  fields: FieldEntry[];
}

export interface FailoverEntry {
  primary: string;
  backup: string;
  strategy: string;
}

export interface EventEntry {
  name: string;
  topic?: string;
  fields: FieldEntry[];
}

export interface SchemaEntry {
  type: string;
  fields: FieldEntry[];
}

/** Emit a list of field dicts as YAML lines. */
export function yamlFields(fields: FieldEntry[], indent = 6): string[] {
  const prefix = ' '.repeat(indent);
  const lines: string[] = [];
  for (const f of fields) {
    lines.push(`${prefix}- name: ${f.name}`);
    lines.push(`${prefix}  type: ${f.type}`);
    if (f.required) {
      lines.push(`${prefix}  required: true`);
    }
    if (f.items_type) {
      lines.push(`${prefix}  items_type: ${f.items_type}`);
    }
  }
  return lines;
}

/** Emit semantic_types section. */
export function yamlSemanticTypes(semanticTypes: SemanticEntry[]): string[] {
  if (semanticTypes.length === 0) return [];
  const lines: string[] = ['semantic_types:'];
  for (const st of semanticTypes) {
    lines.push(`  - name: ${st.name}`);
    lines.push(`    kind: ${st.kind}`);
    if (st.fields.length > 0) {
      lines.push('    fields:');
      lines.push(...yamlFields(st.fields, 6));
    }
    if (st.variants.length > 0) {
      lines.push('    variants:');
      for (const v of st.variants) {
        lines.push(`      - ${v}`);
      }
    }
  }
  return lines;
}

/** Emit errors section. */
export function yamlErrors(errors: ErrorEntry[]): string[] {
  if (errors.length === 0) return [];
  const lines: string[] = ['errors:'];
  for (const err of errors) {
    lines.push(`  - name: ${err.name}`);
    lines.push(`    status: ${err.status}`);
    if (err.code) {
      lines.push(`    code: ${err.code}`);
    }
    if (err.retry !== undefined) {
      lines.push(`    retry: ${err.retry ? 'true' : 'false'}`);
    }
    if (err.fields.length > 0) {
      lines.push('    fields:');
      lines.push(...yamlFields(err.fields, 6));
    }
  }
  return lines;
}

/** Emit state section. */
export function yamlState(state: StateEntry | null): string[] {
  if (!state) return [];
  const lines: string[] = ['state:'];
  lines.push(`  type: ${state.type}`);
  lines.push('  fields:');
  lines.push(...yamlFields(state.fields, 4));
  return lines;
}

/** Emit failover section. */
export function yamlFailover(failoverList: FailoverEntry[]): string[] {
  if (failoverList.length === 0) return [];
  const lines: string[] = ['failover:', '  entries:'];
  for (const e of failoverList) {
    lines.push(`    - primary: ${e.primary}`);
    lines.push(`      backup: ${e.backup}`);
    lines.push(`      strategy: ${e.strategy}`);
  }
  return lines;
}

/** Emit sse_events or ws_events section. */
export function yamlEvents(events: EventEntry[], sectionName: string): string[] {
  if (events.length === 0) return [];
  const lines: string[] = [`${sectionName}:`];
  for (const ev of events) {
    lines.push(`  - name: ${ev.name}`);
    if (ev.topic) {
      lines.push(`    topic: ${ev.topic}`);
    }
    lines.push('    fields:');
    lines.push(...yamlFields(ev.fields, 6));
  }
  return lines;
}

/** Convert a dict of DSL field specs into a normalized schema. */
export function buildSchema(schemaDict: Record<string, any> | null): SchemaEntry | null {
  if (!schemaDict) return null;
  const fields: FieldEntry[] = [];
  for (const [name, spec] of Object.entries(schemaDict)) {
    if (typeof spec === 'object' && spec !== null && spec.type) {
      fields.push({ name, ...spec });
    } else {
      fields.push({ name, type: 'String' });
    }
  }
  return { type: 'json', fields };
}

/** Build a semantic type dict. */
export function makeSemanticEntry(
  name: string,
  kind: string,
  fields?: Record<string, string>,
  variants?: string[],
): SemanticEntry {
  return {
    name,
    kind,
    fields: Object.entries(fields || {}).map(([n, t]) => ({ name: n, type: t })),
    variants: variants || [],
  };
}
