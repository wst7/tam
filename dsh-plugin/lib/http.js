/**
 * Same-origin HTTP API for the tam visual panel.
 *
 * The panel is a browser bundle, so it cannot call the host tools directly.
 * This small JSON API wraps the same `TamClient` the model-facing tools use.
 * It lives under a plugin-owned path, not `/api`, to stay clear of the DSH API
 * proxy's own route tree.
 */

import { TamError } from './core.js';

export const API_PREFIX = '/plugins/dsh-tam/api';
const MAX_BODY_BYTES = 1024 * 1024;

function sendJson(res, status, payload) {
  const body = JSON.stringify(payload);
  res.statusCode = status;
  res.setHeader('content-type', 'application/json; charset=utf-8');
  res.setHeader('cache-control', 'no-store');
  res.end(body);
}

function sendError(res, error) {
  const code = error instanceof TamError ? error.code : 'internal';
  const status = code === 'not_found'
    ? 404
    : code === 'conflict'
      ? 409
      : code === 'invalid'
        ? 400
        : 500;
  sendJson(res, status, {
    ok: false,
    error: {
      code,
      message: error instanceof Error ? error.message : String(error),
      ...error instanceof TamError && error.details !== undefined ? { details: error.details } : {},
    },
  });
}

async function readJsonBody(req) {
  const chunks = [];
  let size = 0;
  for await (const chunk of req) {
    size += chunk.length;
    if (size > MAX_BODY_BYTES) {
      throw new TamError('request body too large', { code: 'invalid' });
    }
    chunks.push(chunk);
  }
  if (chunks.length === 0) return {};
  const text = Buffer.concat(chunks).toString('utf8').trim();
  if (text === '') return {};
  try {
    const value = JSON.parse(text);
    if (value === null || typeof value !== 'object' || Array.isArray(value)) {
      throw new Error('body must be a JSON object');
    }
    return value;
  } catch (error) {
    throw new TamError(`invalid JSON body: ${error instanceof Error ? error.message : String(error)}`, {
      code: 'invalid',
    });
  }
}

function pushFlag(argv, flag, value) {
  if (value === undefined || value === null || value === '') return;
  argv.push(flag, String(value));
}

function parseTaskId(raw) {
  const id = Number(raw);
  if (!Number.isInteger(id) || id < 1) {
    throw new TamError('task id must be a positive integer', { code: 'invalid' });
  }
  return id;
}

function projectOf(url, body) {
  const queryProject = url.searchParams.get('project');
  return queryProject || (typeof body?.project === 'string' ? body.project : undefined);
}

function listArgs(url) {
  const argv = ['task', 'list'];
  pushFlag(argv, '--status', url.searchParams.get('status') ?? undefined);
  pushFlag(argv, '--assigned', url.searchParams.get('assigned') ?? undefined);
  pushFlag(argv, '--priority', url.searchParams.get('priority') ?? undefined);
  pushFlag(argv, '--search', url.searchParams.get('search') ?? undefined);
  pushFlag(argv, '--limit', url.searchParams.get('limit') ?? undefined);
  for (const tag of url.searchParams.getAll('tag')) argv.push('--tag', tag);
  return argv;
}

function createArgs(body) {
  const title = typeof body.title === 'string' ? body.title.trim() : '';
  if (!title) throw new TamError('title is required', { code: 'invalid' });
  const argv = ['task', 'add', title];
  pushFlag(argv, '--description', body.description);
  pushFlag(argv, '--priority', body.priority);
  for (const tag of Array.isArray(body.tags) ? body.tags : []) argv.push('--tag', String(tag));
  pushFlag(argv, '--idempotency-key', body.idempotency_key);
  return argv;
}

function updateArgs(body) {
  const argv = ['task', 'update'];
  pushFlag(argv, '--title', body.title);
  pushFlag(argv, '--description', body.description);
  pushFlag(argv, '--status', body.status);
  pushFlag(argv, '--priority', body.priority);
  if (Object.hasOwn(body, 'tags')) {
    argv.push('--tags', (Array.isArray(body.tags) ? body.tags : []).map(String).join(','));
  }
  return argv;
}

/**
 * Build the HTTP handler bound to one tam client.
 * @param {ReturnType<import('./core.js').createTamClient>} client
 */
export function createTamHttpHandler(client) {
  return async function tamHttpHandler(req, res) {
    try {
      const url = new URL(req.url ?? '/', 'http://localhost');
      const path = url.pathname.slice(API_PREFIX.length) || '/';
      const method = req.method ?? 'GET';

      if (method === 'GET' && path === '/meta') {
        sendJson(res, 200, {
          ok: true,
          data: {
            defaultProject: client.defaultProject,
            defaultAgent: client.defaultAgent,
            command: client.command,
          },
        });
        return;
      }

      if (method === 'GET' && path === '/projects') {
        sendJson(res, 200, { ok: true, data: await client.run(['project', 'list']) });
        return;
      }

      if (method === 'POST' && path === '/projects') {
        const body = await readJsonBody(req);
        if (typeof body.name !== 'string' || body.name.trim() === '') {
          throw new TamError('name is required', { code: 'invalid' });
        }
        sendJson(res, 200, {
          ok: true,
          data: await client.run(['project', 'add', body.name.trim()]),
        });
        return;
      }

      if (method === 'GET' && path === '/tasks') {
        sendJson(res, 200, {
          ok: true,
          data: await client.run(listArgs(url), { project: projectOf(url) }),
        });
        return;
      }

      if (method === 'POST' && path === '/tasks') {
        const body = await readJsonBody(req);
        sendJson(res, 200, {
          ok: true,
          data: await client.run(createArgs(body), {
            project: projectOf(url, body),
            agent: typeof body.agent === 'string' ? body.agent : undefined,
          }),
        });
        return;
      }

      const taskMatch = /^\/tasks\/([^/]+)$/.exec(path);
      if (taskMatch) {
        const id = parseTaskId(decodeURIComponent(taskMatch[1]));
        if (method === 'GET') {
          sendJson(res, 200, {
            ok: true,
            data: await client.run(['task', 'show', String(id)], { project: projectOf(url) }),
          });
          return;
        }
        if (method === 'PATCH' || method === 'PUT') {
          const body = await readJsonBody(req);
          const argv = updateArgs(body);
          argv.splice(2, 0, String(id));
          sendJson(res, 200, {
            ok: true,
            data: await client.run(argv, {
              project: projectOf(url, body),
              agent: typeof body.agent === 'string' ? body.agent : undefined,
            }),
          });
          return;
        }
        if (method === 'DELETE') {
          sendJson(res, 200, {
            ok: true,
            data: await client.run(['task', 'del', String(id)], { project: projectOf(url) }),
          });
          return;
        }
      }

      sendJson(res, 404, {
        ok: false,
        error: { code: 'not_found', message: `no tam API route for ${method} ${path}` },
      });
    } catch (error) {
      sendError(res, error);
    }
  };
}
