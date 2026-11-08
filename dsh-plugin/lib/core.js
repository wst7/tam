/**
 * Pure core for the `dsh-tam` DSH plugin.
 *
 * No DSH imports here: this module owns the tam CLI runner and the tool specs,
 * so it can be unit-tested with plain Node before the plugin is loaded by DSH.
 */

import { execFile } from 'node:child_process';
import { createRequire } from 'node:module';
import { existsSync, realpathSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { promisify } from 'node:util';

const execFileAsync = promisify(execFile);

const MAX_BUFFER = 10 * 1024 * 1024;

/** Business error returned by the tam CLI (a JSON envelope on stderr). */
export class TamError extends Error {
  /**
   * @param {string} message
   * @param {{code?: string, exitCode?: number|null, details?: unknown}} [meta]
   */
  constructor(message, meta = {}) {
    super(message);
    this.name = 'TamError';
    this.code = meta.code ?? 'internal';
    this.exitCode = meta.exitCode ?? null;
    this.details = meta.details;
  }
}

/** Prefer the checkout's freshly built binary for local development. */
export function defaultTamCommand() {
  const exe = process.platform === 'win32' ? 'tam.exe' : 'tam';
  // `dsh plugin add` links this package; resolve symlinks so the checkout's
  // freshly built binary is found in local development.
  const here = dirname(realpathSync(fileURLToPath(import.meta.url)));
  for (const relative of [
    `../../target/debug/${exe}`,
    `../../target/release/${exe}`,
  ]) {
    const candidate = join(here, relative);
    if (existsSync(candidate)) return candidate;
  }
  return 'tam';
}

const require = createRequire(import.meta.url);

/**
 * If the plugin was installed from npm, cargo-dist's `@wst7/tam` package is an
 * optional dependency. Its bin is a launcher that downloads the platform binary
 * on first use; run it through Node so the plugin does not depend on a `tam`
 * shim being on PATH.
 */
function npmTamLauncher() {
  try {
    const launcher = require.resolve('@wst7/tam/run-tam.js');
    // DSH runs inside Electron; ELECTRON_RUN_AS_NODE makes its process.execPath
    // behave as Node for the cargo-dist launcher. Harmless under plain Node.
    return {
      command: process.execPath,
      prefixArgs: [launcher],
      env: { ELECTRON_RUN_AS_NODE: '1' },
    };
  } catch {
    return null;
  }
}

/** Resolve the command used to run tam, in priority order. */
export function resolveTamCommand(config = {}) {
  if (config.command) return { command: config.command, prefixArgs: [] };
  if (process.env.TAM_COMMAND) {
    return { command: process.env.TAM_COMMAND, prefixArgs: [] };
  }
  const npm = npmTamLauncher();
  if (npm) return npm;
  const local = defaultTamCommand();
  return { command: local, prefixArgs: [] };
}

function parseEnvelope(text) {
  const trimmed = text.trim();
  if (!trimmed) return null;
  // stderr may contain a logger line before the envelope; try each line.
  const lines = trimmed.split(/\r?\n/);
  for (let index = lines.length - 1; index >= 0; index -= 1) {
    const line = lines[index].trim();
    if (!line.startsWith('{')) continue;
    try {
      return JSON.parse(line);
    } catch {
      // keep looking
    }
  }
  return null;
}

/**
 * Create a runner bound to one tam binary and optional default context.
 *
 * @param {{
 *   command?: string,
 *   project?: string,
 *   agent?: string,
 *   home?: string,
 *   cwd?: string,
 * }} [config]
 */
export function createTamClient(config = {}) {
  const { command, prefixArgs, env: commandEnv } = resolveTamCommand(config);
  const defaultProject = config.project || process.env.TAM_PROJECT || '';
  const defaultAgent = config.agent || process.env.TAM_AGENT || 'dsh';
  const home = config.home;

  /**
   * Run one tam command in JSON mode and return its `data` payload.
   *
   * @param {string[]} args
   * @param {{project?: string, agent?: string, signal?: AbortSignal}} [options]
   */
  async function run(args, options = {}) {
    const project = options.project || defaultProject;
    const agent = options.agent || defaultAgent;
    const argv = ['--json'];
    if (project) argv.push('--project', project);
    if (agent) argv.push('--agent', agent);
    argv.push(...args);

    const env = { ...process.env, ...commandEnv };
    if (home) {
      env.HOME = home;
      env.USERPROFILE = home;
    }

    try {
      const { stdout } = await execFileAsync(command, [...prefixArgs, ...argv], {
        cwd: config.cwd,
        env,
        maxBuffer: MAX_BUFFER,
        signal: options.signal,
      });
      const envelope = parseEnvelope(stdout);
      if (!envelope) {
        throw new TamError(`tam produced no JSON output for \`${args.join(' ')}\``, {
          code: 'internal',
          exitCode: 1,
        });
      }
      if (envelope.ok !== true) {
        throw new TamError(
          `tam ${args.join(' ')} failed: ${envelope.error?.message ?? 'unknown error'}`,
          {
            code: envelope.error?.code ?? 'internal',
            exitCode: 1,
            details: envelope.error?.details,
          },
        );
      }
      return envelope.data;
    } catch (error) {
      if (error instanceof TamError) throw error;
      if (error?.name === 'AbortError') throw error;

      const exitCode = typeof error?.code === 'number' ? error.code : null;
      const stderr = error?.stderr?.toString?.() ?? '';
      const envelope = parseEnvelope(stderr);
      if (envelope?.error) {
        throw new TamError(
          `tam ${args.join(' ')} failed (exit ${exitCode ?? '?'}, ${envelope.error.code}): ${envelope.error.message}`,
          {
            code: envelope.error.code,
            exitCode,
            details: envelope.error.details,
          },
        );
      }
      throw new TamError(
        `failed to run tam (${command}): ${error?.message ?? String(error)}. ` +
          'Install a tam build with the agent JSON contract (tam >= 1.0.14), or set ' +
          'config.command / TAM_COMMAND to a compatible binary.' +
          (stderr ? `\n${stderr.trim()}` : ''),
        { code: 'internal', exitCode },
      );
    }
  }

  return { run, command, defaultProject, defaultAgent };
}

// ---------------------------------------------------------------------------
// Tool specs
// ---------------------------------------------------------------------------

const objectSchema = { type: 'object', additionalProperties: true };
const arraySchema = {
  type: 'array',
  items: { type: 'object', additionalProperties: true },
};

const renderJson = (_args, value) => [
  { type: 'text', text: JSON.stringify(value, null, 2) },
];

/** Compile the small authoring DSL used below into a raw JSON Schema object. */
export function compileParameters(parameters) {
  const properties = {};
  const required = [];
  for (const [name, node] of Object.entries(parameters)) {
    const { required: isRequired, ...schema } = node;
    properties[name] = schema;
    if (isRequired) required.push(name);
  }
  return {
    type: 'object',
    properties,
    ...(required.length > 0 ? { required } : {}),
    additionalProperties: false,
  };
}

/** Minimal raw-schema validation for tools registered without `defineTool`. */
export function validateArguments(spec, args) {
  if (args === undefined || args === null) args = {};
  if (typeof args !== 'object' || Array.isArray(args)) {
    throw new TamError('tool arguments must be an object', { code: 'invalid' });
  }
  const schema = spec.parametersJsonSchema;
  const known = new Set(Object.keys(schema.properties ?? {}));
  for (const name of Object.keys(args)) {
    if (!known.has(name)) {
      throw new TamError(`unknown argument '${name}'`, { code: 'invalid' });
    }
  }
  for (const name of schema.required ?? []) {
    if (args[name] === undefined) {
      throw new TamError(`missing required argument '${name}'`, { code: 'invalid' });
    }
  }
  for (const [name, node] of Object.entries(schema.properties ?? {})) {
    const value = args[name];
    if (value === undefined) continue;
    if (node.enum && !node.enum.includes(value)) {
      throw new TamError(
        `argument '${name}' must be one of ${node.enum.join(', ')}`,
        { code: 'invalid' },
      );
    }
    switch (node.type) {
      case 'string':
        if (typeof value !== 'string') throw new TamError(`argument '${name}' must be a string`, { code: 'invalid' });
        break;
      case 'number':
      case 'integer':
        if (typeof value !== 'number' || !Number.isFinite(value)) {
          throw new TamError(`argument '${name}' must be a number`, { code: 'invalid' });
        }
        break;
      case 'boolean':
        if (typeof value !== 'boolean') throw new TamError(`argument '${name}' must be a boolean`, { code: 'invalid' });
        break;
      case 'array':
        if (!Array.isArray(value)) throw new TamError(`argument '${name}' must be an array`, { code: 'invalid' });
        if (node.items?.type === 'string' && value.some((item) => typeof item !== 'string')) {
          throw new TamError(`argument '${name}' must contain only strings`, { code: 'invalid' });
        }
        break;
      default:
        break;
    }
  }
  return args;
}

const PROJECT_PARAM = {
  type: 'string',
  description:
    'tam project name. Required when multiple projects exist; otherwise defaults to plugin config/TAM_PROJECT.',
};
const AGENT_PARAM = {
  type: 'string',
  description:
    'Agent identity for leases/events. Defaults to plugin config/TAM_AGENT/dsh; pass explicitly for multi-agent work.',
};

function taskId(args) {
  if (typeof args.id !== 'number' || !Number.isInteger(args.id) || args.id < 1) {
    throw new TamError('id must be a positive integer', { code: 'invalid' });
  }
  return String(args.id);
}

function pushFlag(argv, flag, value) {
  if (value === undefined || value === null || value === '') return;
  argv.push(flag, String(value));
}

function pushTags(argv, tags) {
  for (const tag of tags ?? []) {
    argv.push('--tag', String(tag));
  }
}

/**
 * One model-facing tool. `execute(args, exec, client)` returns the tam `data`
 * value; throwing makes the DSH tool result an error.
 */
function spec(name, description, parameters, outputSchema, execute) {
  return {
    name,
    description,
    parameters,
    parametersJsonSchema: compileParameters(parameters),
    outputSchema,
    render: renderJson,
    execute,
  };
}

export const toolSpecs = [
  spec(
    'tam_status',
    'Show task counts, leases, unassigned work, and next claimable tasks for a tam project. Pass project in multi-project setups.',
    { project: PROJECT_PARAM, agent: AGENT_PARAM },
    objectSchema,
    (args, exec, client) =>
      client.run(['status'], {
        project: args.project,
        agent: args.agent,
        signal: exec.signal,
      }),
  ),
  spec(
    'tam_project_list',
    'List all active tam projects. Use this to discover project names before creating/listing tasks.',
    {},
    arraySchema,
    (_args, exec, client) => client.run(['project', 'list'], { signal: exec.signal }),
  ),
  spec(
    'tam_project_add',
    'Create a tam project (or restore a soft-deleted one).',
    {
      name: { type: 'string', required: true, description: 'New project name' },
    },
    objectSchema,
    (args, exec, client) =>
      client.run(['project', 'add', String(args.name)], { signal: exec.signal }),
  ),
  spec(
    'tam_task_list',
    'List tasks in a tam project with optional status/assignee/priority/tag/search/limit filters. Pass project in multi-project setups.',
    {
      project: PROJECT_PARAM,
      agent: AGENT_PARAM,
      status: {
        type: 'string',
        enum: ['todo', 'in-progress', 'done'],
        description: 'Only tasks with this status',
      },
      assigned: {
        type: 'string',
        description: '`me`, `unassigned`, or an agent id',
      },
      priority: { type: 'number', description: 'Only this priority (1 highest .. 5 lowest)' },
      tags: {
        type: 'array',
        items: { type: 'string' },
        description: 'Only tasks carrying all of these tags',
      },
      search: { type: 'string', description: 'Case-insensitive title/description substring' },
      limit: { type: 'number', description: 'Max tasks (default 100, max 1000)' },
    },
    objectSchema,
    (args, exec, client) => {
      const argv = ['task', 'list'];
      pushFlag(argv, '--status', args.status);
      pushFlag(argv, '--assigned', args.assigned);
      pushFlag(argv, '--priority', args.priority);
      pushTags(argv, args.tags);
      pushFlag(argv, '--search', args.search);
      pushFlag(argv, '--limit', args.limit);
      return client.run(argv, {
        project: args.project,
        agent: args.agent,
        signal: exec.signal,
      });
    },
  ),
  spec(
    'tam_task_show',
    'Show one tam task with its audit events.',
    {
      id: { type: 'number', required: true, description: 'Task id' },
      project: PROJECT_PARAM,
      agent: AGENT_PARAM,
    },
    objectSchema,
    (args, exec, client) =>
      client.run(['task', 'show', taskId(args)], {
        project: args.project,
        agent: args.agent,
        signal: exec.signal,
      }),
  ),
  spec(
    'tam_task_create',
    'Create a tam task. Use idempotency_key for safe retries; the CLI returns deduplicated:true when the key already exists.',
    {
      title: { type: 'string', required: true, description: 'Task title' },
      project: PROJECT_PARAM,
      agent: AGENT_PARAM,
      description: { type: 'string', description: 'Task description/context' },
      priority: { type: 'number', description: '1 highest .. 5 lowest (default 3)' },
      tags: {
        type: 'array',
        items: { type: 'string' },
        description: 'Tags (normalized lowercase)',
      },
      idempotency_key: {
        type: 'string',
        description: 'Project-scoped retry key; duplicate calls return the same task',
      },
    },
    objectSchema,
    (args, exec, client) => {
      const argv = ['task', 'add', String(args.title)];
      pushFlag(argv, '--description', args.description);
      pushFlag(argv, '--priority', args.priority);
      pushTags(argv, args.tags);
      pushFlag(argv, '--idempotency-key', args.idempotency_key);
      return client.run(argv, {
        project: args.project,
        agent: args.agent,
        signal: exec.signal,
      });
    },
  ),
  spec(
    'tam_task_update',
    'Partially update a tam task (title/description/status/priority/tags). Only the lease holder or a human caller may update a leased task.',
    {
      id: { type: 'number', required: true, description: 'Task id' },
      project: PROJECT_PARAM,
      agent: AGENT_PARAM,
      title: { type: 'string' },
      description: { type: 'string' },
      status: {
        type: 'string',
        enum: ['todo', 'in-progress', 'done'],
        description: 'New status; todo/done clear the lease',
      },
      priority: { type: 'number', description: '1 highest .. 5 lowest' },
      tags: {
        type: 'array',
        items: { type: 'string' },
        description: 'Replace tags; pass [] to clear',
      },
    },
    objectSchema,
    (args, exec, client) => {
      const argv = ['task', 'update', taskId(args)];
      pushFlag(argv, '--title', args.title);
      pushFlag(argv, '--description', args.description);
      pushFlag(argv, '--status', args.status);
      pushFlag(argv, '--priority', args.priority);
      if (args.tags !== undefined) {
        argv.push('--tags', (args.tags ?? []).map(String).join(','));
      }
      return client.run(argv, {
        project: args.project,
        agent: args.agent,
        signal: exec.signal,
      });
    },
  ),
  spec(
    'tam_task_claim',
    'Atomically claim a tam task with a lease. Another agent holding a valid lease causes a conflict (exit 4).',
    {
      id: { type: 'number', required: true, description: 'Task id' },
      project: PROJECT_PARAM,
      agent: AGENT_PARAM,
      lease: { type: 'number', description: 'Lease seconds (default 900, 5..86400)' },
      force: { type: 'boolean', description: 'Preempt another agent lease (recorded in events)' },
    },
    objectSchema,
    (args, exec, client) => {
      const argv = ['task', 'claim', taskId(args)];
      pushFlag(argv, '--lease', args.lease);
      if (args.force) argv.push('--force');
      return client.run(argv, {
        project: args.project,
        agent: args.agent,
        signal: exec.signal,
      });
    },
  ),
  spec(
    'tam_task_heartbeat',
    'Renew the lease of a task held by the caller. Only the holder can heartbeat.',
    {
      id: { type: 'number', required: true, description: 'Task id' },
      project: PROJECT_PARAM,
      agent: AGENT_PARAM,
      lease: { type: 'number', description: 'New lease seconds (default 900)' },
    },
    objectSchema,
    (args, exec, client) => {
      const argv = ['task', 'heartbeat', taskId(args)];
      pushFlag(argv, '--lease', args.lease);
      return client.run(argv, {
        project: args.project,
        agent: args.agent,
        signal: exec.signal,
      });
    },
  ),
  spec(
    'tam_task_release',
    'Release a task lease and return an in-progress task to todo.',
    {
      id: { type: 'number', required: true, description: 'Task id' },
      project: PROJECT_PARAM,
      agent: AGENT_PARAM,
      force: { type: 'boolean', description: 'Release even when another agent holds the lease' },
    },
    objectSchema,
    (args, exec, client) => {
      const argv = ['task', 'release', taskId(args)];
      if (args.force) argv.push('--force');
      return client.run(argv, {
        project: args.project,
        agent: args.agent,
        signal: exec.signal,
      });
    },
  ),
  spec(
    'tam_task_complete',
    'Mark a task done, clear its lease, and append a completed event.',
    {
      id: { type: 'number', required: true, description: 'Task id' },
      project: PROJECT_PARAM,
      agent: AGENT_PARAM,
    },
    objectSchema,
    (args, exec, client) =>
      client.run(['task', 'done', taskId(args)], {
        project: args.project,
        agent: args.agent,
        signal: exec.signal,
      }),
  ),
  spec(
    'tam_task_next',
    'Return the next claimable task (priority ascending, created ascending). Set claim=true to atomically claim it.',
    {
      project: PROJECT_PARAM,
      agent: AGENT_PARAM,
      claim: { type: 'boolean', description: 'Atomically claim the returned task' },
      lease: { type: 'number', description: 'Lease seconds when claiming (default 900)' },
    },
    objectSchema,
    (args, exec, client) => {
      const argv = ['task', 'next'];
      if (args.claim) argv.push('--claim');
      pushFlag(argv, '--lease', args.lease);
      return client.run(argv, {
        project: args.project,
        agent: args.agent,
        signal: exec.signal,
      });
    },
  ),
  spec(
    'tam_task_events',
    'Show the audit event log for one task.',
    {
      id: { type: 'number', required: true, description: 'Task id' },
      project: PROJECT_PARAM,
      agent: AGENT_PARAM,
    },
    objectSchema,
    (args, exec, client) =>
      client.run(['task', 'events', taskId(args)], {
        project: args.project,
        agent: args.agent,
        signal: exec.signal,
      }),
  ),
  spec(
    'tam_task_delete',
    'Soft-delete a task and append a deleted event.',
    {
      id: { type: 'number', required: true, description: 'Task id' },
      project: PROJECT_PARAM,
      agent: AGENT_PARAM,
    },
    objectSchema,
    (args, exec, client) =>
      client.run(['task', 'del', taskId(args)], {
        project: args.project,
        agent: args.agent,
        signal: exec.signal,
      }),
  ),
];
