/**
 * DSH host plugin: expose tam as native model-facing tools.
 *
 * Registers raw `ToolDefinition`s on Cordis's injected `tools` service. There
 * is deliberately no `@deepseek-ai/dsh-tools` import: a linked bundle can be
 * loaded from a hoisted profile without resolving DSH peers, and the tool
 * specs/validation live in testable plain Node code (`lib/core.js`).
 */

import { createTamClient, toolSpecs, validateArguments } from './lib/core.js';
import { API_PREFIX, createTamHttpHandler } from './lib/http.js';

/** Cordis plugin name (diagnostics only). */
export const name = 'tam-tools';

/** Wait for the tool registry before registering. */
export const inject = ['tools'];

/**
 * @param {import('@deepseek-ai/cordis').Context} ctx
 * @param {{command?: string, project?: string, agent?: string, cwd?: string}} [config]
 */
export function apply(ctx, config = {}) {
  const client = createTamClient(config);

  // The web composition loads webServer asynchronously; wait for it instead
  // of reading it once. Headless profiles simply never provide this service.
  ctx.inject(['webServer'], (webCtx) => {
    webCtx.effect(
      () => webCtx.webServer.register({
        kind: 'prefix',
        path: API_PREFIX,
        handler: createTamHttpHandler(client),
      }),
      'dsh-tam: visual panel HTTP API',
    );
  });

  for (const spec of toolSpecs) {
    ctx.tools.register({
      name: spec.name,
      description: spec.description,
      parameters: spec.parametersJsonSchema,
      output: {
        schema: spec.outputSchema,
        render: spec.render,
      },
      execute(args, exec) {
        return spec.execute(validateArguments(spec, args), exec, client);
      },
    });
  }
}
