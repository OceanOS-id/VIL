/**
 * VilPipeline — SSE Pipeline DSL (HttpSink + HttpSource + Tri-Lane)
 *
 * Generates a YAML manifest with pipeline: section (nodes + routes).
 * `vil compile` transpiles it to a native Rust binary using VIL Way
 * patterns (ServiceCtx, ShmSlice).
 */

import { execFileSync } from 'child_process';
import * as fs from 'fs';
import * as os from 'os';
import * as path from 'path';
import {
  FieldEntry,
  SemanticEntry,
  StateEntry,
  FailoverEntry,
  EventEntry,
  yamlSemanticTypes,
  yamlErrors,
  yamlState,
  yamlFailover,
  yamlEvents,
  yamlFields,
  makeSemanticEntry,
  ErrorEntry,
} from './yaml';

interface NodeEntry {
  type: string;
  port?: number;
  path?: string;
  url?: string;
  format?: string;
  json_tap?: string;
  post_body?: string;
  dialect?: string;
  code?: { mode: string; body: string };
}

interface RouteEntry {
  from: string;
  to: string;
  mode: string;
}

export class VilPipeline {
  name: string;
  port: number;
  token = 'shm';
  private _nodes: Map<string, NodeEntry> = new Map();
  private _routes: RouteEntry[] = [];
  private _semanticTypes: SemanticEntry[] = [];
  private _errors: ErrorEntry[] = [];
  private _state: StateEntry | null = null;
  private _failover: FailoverEntry[] = [];
  private _sseEvents: EventEntry[] = [];
  private _wsEvents: EventEntry[] = [];

  constructor(name: string, port = 3080) {
    this.name = name;
    this.port = port;
  }

  // -- Node builders --------------------------------------------------------

  /** Add an HttpSink node (webhook trigger endpoint). */
  sink(opts: { port?: number; path?: string; name?: string } = {}): this {
    const nodeName = opts.name || 'http_sink';
    const nodePort = opts.port ?? 3080;
    const nodePath = opts.path ?? '/trigger';
    this._nodes.set(nodeName, {
      type: 'http_sink',
      port: nodePort,
      path: nodePath,
    });
    return this;
  }

  /** Add an HttpSource node (upstream inference endpoint). */
  source(opts: {
    url?: string;
    format?: string;
    name?: string;
    jsonTap?: string;
    postBody?: string;
    dialect?: string;
  }): this {
    const nodeName = opts.name || 'http_source';
    const node: NodeEntry = {
      type: 'http_source',
      url: opts.url,
      format: opts.format,
    };
    if (opts.jsonTap) node.json_tap = opts.jsonTap;
    if (opts.postBody) node.post_body = opts.postBody;
    if (opts.dialect) node.dialect = opts.dialect;
    this._nodes.set(nodeName, node);
    return this;
  }

  /** Add a transform node with inline code. */
  transform(name: string, fnBody?: string): this {
    const node: NodeEntry = { type: 'transform' };
    if (fnBody) {
      node.code = { mode: 'expr', body: fnBody };
    }
    this._nodes.set(name, node);
    return this;
  }

  /** Add a route between node ports. */
  route(srcPort: string, dstPort: string, mode = 'LoanWrite'): this {
    this._routes.push({ from: srcPort, to: dstPort, mode });
    return this;
  }

  // -- Semantic type declarations -------------------------------------------

  /** Declare a semantic type (state/event/fault/decision). */
  semanticType(
    name: string,
    kind: string,
    fields?: Record<string, string>,
    variants?: string[],
  ): this {
    this._semanticTypes.push(makeSemanticEntry(name, kind, fields, variants));
    return this;
  }

  /** Shorthand: declare a semantic state type AND set service state. */
  state(name: string, fields: Record<string, string>): this {
    this.semanticType(name, 'state', fields);
    this._state = {
      type: 'private_heap',
      fields: Object.entries(fields).map(([n, t]) => ({ name: n, type: t })),
    };
    return this;
  }

  /** Shorthand: declare a semantic event type. */
  event(name: string, fields: Record<string, string>): this {
    return this.semanticType(name, 'event', fields);
  }

  /** Shorthand: declare a semantic fault type. */
  fault(name: string, variants?: string[]): this {
    return this.semanticType(name, 'fault', undefined, variants || []);
  }

  /** Shorthand: declare a semantic decision type. */
  decision(name: string, fields: Record<string, string>): this {
    return this.semanticType(name, 'decision', fields);
  }

  /** Declare a failover entry. */
  failover(primary: string, backup: string, strategy = 'immediate'): this {
    this._failover.push({ primary, backup, strategy });
    return this;
  }

  /** Declare an SSE event type. */
  sseEvent(name: string, fields: Record<string, string>, topic?: string): this {
    this._sseEvents.push({
      name,
      topic,
      fields: Object.entries(fields).map(([n, t]) => ({ name: n, type: t })),
    });
    return this;
  }

  /** Declare a WebSocket event type. */
  wsEvent(name: string, topic?: string, fields?: Record<string, string>): this {
    this._wsEvents.push({
      name,
      topic,
      fields: Object.entries(fields || {}).map(([n, t]) => ({ name: n, type: t })),
    });
    return this;
  }

  // -- YAML generation ------------------------------------------------------

  /** Generate YAML manifest string for `vil compile`. */
  toYaml(): string {
    const lines: string[] = [];
    lines.push('vil_version: "6.0.0"');
    lines.push(`name: ${this.name}`);
    lines.push(`port: ${this.port}`);
    lines.push(`token: ${this.token}`);

    lines.push(...yamlSemanticTypes(this._semanticTypes));
    lines.push(...yamlErrors(this._errors));
    lines.push(...yamlState(this._state));
    lines.push(...yamlFailover(this._failover));
    lines.push(...yamlEvents(this._sseEvents, 'sse_events'));
    lines.push(...yamlEvents(this._wsEvents, 'ws_events'));

    // Nodes
    if (this._nodes.size > 0) {
      lines.push('');
      lines.push('nodes:');
      for (const [nodeName, node] of this._nodes) {
        lines.push(`  ${nodeName}:`);
        lines.push(`    type: ${node.type}`);
        if (node.port) {
          lines.push(`    port: ${node.port}`);
        }
        if (node.path) {
          lines.push(`    path: "${node.path}"`);
        }
        if (node.url) {
          lines.push(`    url: "${node.url}"`);
        }
        if (node.format) {
          lines.push(`    format: ${node.format}`);
        }
        if (node.json_tap) {
          lines.push(`    json_tap: "${node.json_tap}"`);
        }
        if (node.dialect) {
          lines.push(`    dialect: ${node.dialect}`);
        }
        if (node.post_body) {
          lines.push(`    post_body: ${JSON.stringify(node.post_body)}`);
        }
        if (node.code) {
          lines.push('    code:');
          lines.push(`      mode: ${node.code.mode}`);
          lines.push(`      body: "${node.code.body}"`);
        }
      }
    }

    // Routes
    if (this._routes.length > 0) {
      lines.push('');
      lines.push('routes:');
      for (const r of this._routes) {
        lines.push(`  - from: ${r.from}`);
        lines.push(`    to: ${r.to}`);
        lines.push(`    mode: ${r.mode}`);
      }
    }

    return lines.join('\n') + '\n';
  }

  /**
   * Call `vil compile` with the generated YAML manifest.
   * If VIL_COMPILE_MODE=manifest, print YAML to stdout and exit.
   */
  compile(release = true): void {
    if (process.env.VIL_COMPILE_MODE === 'manifest') {
      process.stdout.write(this.toYaml());
      return;
    }

    // Write manifest to temp file and invoke vil compile
    const tmpDir = os.tmpdir();
    const manifestPath = path.join(tmpDir, `vil-${this.name}-${Date.now()}.yaml`);
    fs.writeFileSync(manifestPath, this.toYaml(), 'utf-8');

    const cmd = ['vil', 'compile', '--manifest', manifestPath];
    if (release) {
      cmd.push('--release');
    }
    cmd.push('--output', this.name);

    console.log(`  Compiling pipeline: ${this.name}`);
    console.log(`  Command: ${cmd.join(' ')}`);
    try {
      execFileSync(cmd[0], cmd.slice(1), { stdio: 'inherit' });
    } catch (err: any) {
      if (err.code === 'ENOENT') {
        console.log("\n  'vil' CLI not found. Install with: cargo install vil_cli");
        console.log(`  Manifest written to: ${manifestPath}`);
      } else {
        console.log(`\n  Compilation failed (exit code ${err.status})`);
        console.log(`  Manifest: ${manifestPath}`);
      }
    }
  }
}
