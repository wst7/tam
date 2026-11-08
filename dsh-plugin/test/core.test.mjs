import assert from 'node:assert/strict';
import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import test from 'node:test';

import { createTamClient, resolveTamCommand, TamError, toolSpecs, validateArguments } from '../lib/core.js';

const TAM_BIN =
  process.env.TAM_BIN ||
  fileURLToPath(new URL('../../target/debug/tam', import.meta.url));

function makeClient() {
  const home = mkdtempSync(join(tmpdir(), 'dsh-tam-test-'));
  const client = createTamClient({
    command: TAM_BIN,
    home,
    project: 'p1',
    agent: 'dsh-test',
  });
  return { client, home };
}

function cleanup(home) {
  rmSync(home, { recursive: true, force: true });
}

function spec(name) {
  const found = toolSpecs.find((entry) => entry.name === name);
  assert.ok(found, `missing tool spec ${name}`);
  return found;
}

function runTool(name, args, client) {
  const controller = new AbortController();
  return spec(name).execute(args, { signal: controller.signal }, client);
}

test('project/task CRUD through native tool specs', async (t) => {
  const { client, home } = makeClient();
  t.after(() => cleanup(home));

  const projects = await runTool('tam_project_list', {}, client);
  assert.deepEqual(projects, []);

  const p1 = await runTool('tam_project_add', { name: 'p1' }, client);
  assert.equal(p1.project.name, 'p1');
  await runTool('tam_project_add', { name: 'p2' }, client);

  const created = await runTool(
    'tam_task_create',
    {
      project: 'p1',
      title: 'fix login',
      description: 'agent context',
      priority: 1,
      tags: ['Bug', 'backend'],
    },
    client,
  );
  assert.equal(created.task.project, 'p1');
  assert.equal(created.task.priority, 1);
  assert.deepEqual(created.task.tags, ['bug', 'backend']);

  const p2Task = await runTool(
    'tam_task_create',
    { project: 'p2', title: 'p2 task' },
    client,
  );
  assert.equal(p2Task.task.project, 'p2');

  const p1List = await runTool('tam_task_list', {}, client);
  assert.equal(p1List.count, 1);
  assert.equal(p1List.tasks[0].id, created.task.id);

  const p2List = await runTool('tam_task_list', { project: 'p2' }, client);
  assert.equal(p2List.count, 1);
  assert.equal(p2List.tasks[0].id, p2Task.task.id);

  const shown = await runTool(
    'tam_task_show',
    { project: 'p1', id: created.task.id },
    client,
  );
  assert.equal(shown.task.title, 'fix login');
  assert.equal(shown.events[0].event, 'created');

  const status = await runTool('tam_status', { project: 'p1' }, client);
  assert.equal(status.counts.todo, 1);

  const deleted = await runTool(
    'tam_task_delete',
    { project: 'p2', id: p2Task.task.id },
    client,
  );
  assert.equal(deleted.task.id, p2Task.task.id);
  const p2After = await runTool('tam_task_list', { project: 'p2' }, client);
  assert.equal(p2After.count, 0);
});

test('claim conflict, heartbeat, release, complete and events', async (t) => {
  const { client, home } = makeClient();
  t.after(() => cleanup(home));

  await runTool('tam_project_add', { name: 'p1' }, client);
  const created = await runTool(
    'tam_task_create',
    { project: 'p1', title: 'lease me', priority: 1 },
    client,
  );
  const id = created.task.id;

  const claimed = await runTool(
    'tam_task_claim',
    { project: 'p1', id, agent: 'agent-a', lease: 60 },
    client,
  );
  assert.equal(claimed.task.status, 'in-progress');
  assert.equal(claimed.task.assigned_agent, 'agent-a');

  await assert.rejects(
    runTool('tam_task_claim', { project: 'p1', id, agent: 'agent-b' }, client),
    (error) =>
      error instanceof TamError &&
      error.code === 'conflict' &&
      error.exitCode === 4,
  );

  const heartbeat = await runTool(
    'tam_task_heartbeat',
    { project: 'p1', id, agent: 'agent-a', lease: 120 },
    client,
  );
  assert.ok(heartbeat.task.revision >= claimed.task.revision);

  const released = await runTool(
    'tam_task_release',
    { project: 'p1', id, agent: 'agent-a' },
    client,
  );
  assert.equal(released.task.status, 'todo');
  assert.equal(released.task.assigned_agent, null);

  const reclaimed = await runTool(
    'tam_task_claim',
    { project: 'p1', id, agent: 'agent-b', lease: 60 },
    client,
  );
  assert.equal(reclaimed.task.assigned_agent, 'agent-b');

  const completed = await runTool(
    'tam_task_complete',
    { project: 'p1', id, agent: 'agent-b' },
    client,
  );
  assert.equal(completed.task.status, 'done');
  assert.equal(completed.task.assigned_agent, null);

  const { events } = await runTool(
    'tam_task_events',
    { project: 'p1', id },
    client,
  );
  const names = events.map((event) => event.event);
  for (const expected of ['created', 'claimed', 'heartbeat', 'released', 'completed']) {
    assert.ok(names.includes(expected), `missing ${expected}: ${names.join(',')}`);
  }
});

test('idempotency key returns the same task', async (t) => {
  const { client, home } = makeClient();
  t.after(() => cleanup(home));
  await runTool('tam_project_add', { name: 'p1' }, client);

  const first = await runTool(
    'tam_task_create',
    { project: 'p1', title: 'retry me', idempotency_key: 'retry-1' },
    client,
  );
  assert.equal(first.deduplicated, false);

  const second = await runTool(
    'tam_task_create',
    { project: 'p1', title: 'retry me again', idempotency_key: 'retry-1' },
    client,
  );
  assert.equal(second.deduplicated, true);
  assert.equal(second.task.id, first.task.id);
});

test('next claims highest priority and project isolation', async (t) => {
  const { client, home } = makeClient();
  t.after(() => cleanup(home));
  await runTool('tam_project_add', { name: 'p1' }, client);
  await runTool('tam_project_add', { name: 'p2' }, client);

  const low = await runTool(
    'tam_task_create',
    { project: 'p1', title: 'low', priority: 4 },
    client,
  );
  const high = await runTool(
    'tam_task_create',
    { project: 'p1', title: 'high', priority: 1 },
    client,
  );
  const other = await runTool(
    'tam_task_create',
    { project: 'p2', title: 'other project' },
    client,
  );

  const next = await runTool(
    'tam_task_next',
    { project: 'p1', agent: 'dsh-test', claim: true, lease: 60 },
    client,
  );
  assert.equal(next.task.id, high.task.id);
  assert.equal(next.task.status, 'in-progress');

  await assert.rejects(
    runTool(
      'tam_task_show',
      { project: 'p1', id: other.task.id },
      client,
    ),
    (error) => error instanceof TamError && error.code === 'not_found',
  );

  const todo = await runTool('tam_task_list', { project: 'p1', status: 'todo' }, client);
  assert.equal(todo.count, 1);
  assert.equal(todo.tasks[0].id, low.task.id);
});


test('raw tool argument validation', () => {
  const list = toolSpecs.find((entry) => entry.name === 'tam_task_list');
  assert.throws(
    () => validateArguments(list, { status: 123 }),
    (error) => error instanceof TamError && error.code === 'invalid',
  );
  assert.throws(
    () => validateArguments(list, { unknown_field: true }),
    (error) => error instanceof TamError && error.code === 'invalid',
  );
  const args = validateArguments(list, { status: 'todo', tags: ['bug'], limit: 10 });
  assert.equal(args.status, 'todo');
  assert.deepEqual(args.tags, ['bug']);
});


test('command resolution: explicit config wins over environment', () => {
  const previous = process.env.TAM_COMMAND;
  process.env.TAM_COMMAND = '/env/tam';
  try {
    assert.deepEqual(resolveTamCommand({ command: '/config/tam' }), {
      command: '/config/tam',
      prefixArgs: [],
    });
    assert.deepEqual(resolveTamCommand(), {
      command: '/env/tam',
      prefixArgs: [],
    });
  } finally {
    if (previous === undefined) delete process.env.TAM_COMMAND;
    else process.env.TAM_COMMAND = previous;
  }
});
