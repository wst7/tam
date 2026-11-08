import assert from 'node:assert/strict';
import { createServer } from 'node:http';
import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import test from 'node:test';

import { createTamClient } from '../lib/core.js';
import { API_PREFIX, createTamHttpHandler } from '../lib/http.js';

const TAM_BIN =
  process.env.TAM_BIN ||
  fileURLToPath(new URL('../../target/debug/tam', import.meta.url));

async function withApi(fn) {
  const home = mkdtempSync(join(tmpdir(), 'dsh-tam-http-'));
  const client = createTamClient({
    command: TAM_BIN,
    home,
    project: 'p1',
    agent: 'panel-test',
  });
  const server = createServer(createTamHttpHandler(client));
  await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve));
  const address = server.address();
  const base = `http://127.0.0.1:${address.port}${API_PREFIX}`;
  try {
    await fn(base);
  } finally {
    await new Promise((resolve) => server.close(resolve));
    rmSync(home, { recursive: true, force: true });
  }
}

async function call(base, path, init = {}) {
  const response = await fetch(`${base}${path}`, {
    headers: { 'content-type': 'application/json' },
    ...init,
  });
  const body = await response.json();
  assert.equal(body.ok, true, JSON.stringify(body));
  return body.data;
}

test('panel HTTP API: meta, projects, task CRUD', async () => {
  await withApi(async (base) => {
    const meta = await call(base, '/meta');
    assert.equal(meta.defaultProject, 'p1');
    assert.equal(meta.defaultAgent, 'panel-test');

    const project = await call(base, '/projects', {
      method: 'POST',
      body: JSON.stringify({ name: 'p1' }),
    });
    assert.equal(project.project.name, 'p1');

    const created = await call(base, '/tasks', {
      method: 'POST',
      body: JSON.stringify({
        project: 'p1',
        title: 'panel task',
        description: 'manual edit',
        priority: 1,
        tags: ['Panel', 'UI'],
      }),
    });
    assert.equal(created.task.title, 'panel task');
    assert.deepEqual(created.task.tags, ['panel', 'ui']);
    const id = created.task.id;

    const listed = await call(base, '/tasks?project=p1');
    assert.equal(listed.count, 1);
    assert.equal(listed.tasks[0].id, id);

    const updated = await call(base, `/tasks/${id}?project=p1`, {
      method: 'PATCH',
      body: JSON.stringify({
        title: 'edited title',
        description: 'edited description',
        status: 'in-progress',
        priority: 4,
        tags: ['edited'],
      }),
    });
    assert.equal(updated.task.title, 'edited title');
    assert.equal(updated.task.status, 'in-progress');
    assert.equal(updated.task.priority, 4);
    assert.deepEqual(updated.task.tags, ['edited']);

    const shown = await call(base, `/tasks/${id}?project=p1`);
    assert.equal(shown.task.title, 'edited title');

    const removed = await call(base, `/tasks/${id}?project=p1`, { method: 'DELETE' });
    assert.equal(removed.task.id, id);

    const empty = await call(base, '/tasks?project=p1');
    assert.equal(empty.count, 0);
  });
});

test('panel HTTP API: missing project/task errors keep status semantics', async () => {
  await withApi(async (base) => {
    const taskResponse = await fetch(`${base}/tasks/99?project=p1`, {
      headers: { 'content-type': 'application/json' },
    });
    const taskBody = await taskResponse.json();
    assert.equal(taskResponse.status, 404);
    assert.equal(taskBody.ok, false);
    assert.equal(taskBody.error.code, 'not_found');
  });
});
