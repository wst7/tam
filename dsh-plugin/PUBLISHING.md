# Publishing `@wst7/dsh-tam`

This package is a normal npm package with a DSH bundle manifest. It has an **optional
dependency** on `@wst7/tam` (`>=1.0.14 <2.0.0`), so installing the plugin from npm also
installs the CLI when that tam version is available. The plugin still requires a tam build
with the agent JSON contract (`tam --json project list` returns `tam.agent/v1`).

**Release tam first**, then the plugin. If the plugin is published before tam 1.0.14 is
on npm, `dsh plugin add @wst7/dsh-tam` still succeeds but pnpm skips the optional dependency;
users then need a separate tam install (or `command` config) until they reinstall.

## Versioning

- `@wst7/dsh-tam` follows semver: patch = fixes, minor = new tools/behavior, major = breaking
  tool/config changes.
- The tool contract is the tam JSON contract `tam.agent/v1`. If tam introduces
  `tam.agent/v2`, publish a plugin major and support both or require the new tam.
- Keep the plugin version independent from `tam`'s crate version; document the required
  tam range in the GitHub release notes and README.

## Pre-flight

```bash
cd dsh-plugin
node --test test/*.test.mjs        # requires cargo build (uses target/debug/tam)
npm pack --dry-run                 # confirm only runtime files are packed
npm publish --dry-run              # validate metadata/access
```

Expected tarball contains `README.md`, `PUBLISHING.md`, `cordis.patch.yml`, `index.js`,
`client.js`, `lib/core.js`, `lib/http.js`, `package.json`; tests are excluded by `files`.
`client.js` is a prebuilt DSH client bundle (classic script registering
`window.__ModuleLoader__`), so publishing needs no client build.

Check the registry before publishing (in China the default may be a mirror that does not
accept publishes):

```bash
npm config get registry
# use npmjs for the public package:
npm publish --registry=https://registry.npmjs.org --access public
```

`@wst7/dsh-tam` is published under the existing `@wst7` npm scope, so the
`NPM_TOKEN` used for `@wst7/tam` can publish it too. If the package is renamed,
update `package.json`, the bundle row in `cordis.patch.yml`, and the
`__ModuleLoader__.load` id in `client.js` together.

## Release order

1. Release tam (next version, at least 1.0.14) so cargo-dist publishes
   `@wst7/tam@1.0.14` (or newer) to npm.
2. Verify the npm package and its launcher:
   ```bash
   npm view @wst7/tam version
   npx @wst7/tam@latest --version
   ```
3. Publish the plugin. A clean install should pull both packages:
   ```bash
   dsh plugin --profile desktop add @wst7/dsh-tam@0.1.1
   grep -n '@wst7/tam' ~/.dsh/profiles/desktop/package.json
   ```
   The first tool call may download the platform binary via the launcher.

If the next tam release is not 1.0.14, update `optionalDependencies` in
`dsh-plugin/package.json` to the actual release (`>=<version> <2.0.0`).

## Release

1. Bump the plugin version without creating an unwanted repo tag:

   ```bash
   cd dsh-plugin
   npm version patch --no-git-tag-version
   ```

2. Commit the version bump and tag it (tag prefix keeps it separate from Rust releases):

   ```bash
   git add dsh-plugin/package.json dsh-plugin/README.md
   git commit -m "chore(dsh-plugin): release 0.1.1"
   git tag dsh-plugin-v0.1.1
   git push origin main --tags
   ```

3. Publish to npm:

   ```bash
   cd dsh-plugin
   npm publish --registry=https://registry.npmjs.org --access public
   ```

4. Optional: attach a tarball to the GitHub release for offline/enterprise users:

   ```bash
   npm pack
   sha256sum wst7-dsh-tam-0.1.1.tgz
   gh release create dsh-plugin-v0.1.1 wst7-dsh-tam-0.1.1.tgz \
     --title "@wst7/dsh-tam 0.1.1" \
     --notes "Requires tam with tam.agent/v1."
   ```

   Users install it with:

   ```bash
   dsh plugin --profile desktop add ./wst7-dsh-tam-0.1.1.tgz
   # or a URL
   dsh plugin --profile desktop add https://github.com/wst7/tam/releases/download/dsh-plugin-v0.1.1/wst7-dsh-tam-0.1.1.tgz
   ```

5. Verify a clean consumer install:

   ```bash
   dsh plugin --profile desktop add @wst7/dsh-tam@0.1.1
   # restart DeepSeek Harness, then confirm the tam_* tools appear
   dsh plugin --profile desktop remove @wst7/dsh-tam
   ```

## CI (optional)

Publishing from CI needs an npm token that can **create** packages in the `@wst7`
scope. A granular token restricted to existing packages authenticates but answers
`404 ... is not in this registry` on the first publish of a new package; create a new
granular token with package read/write access for the scope (or publish from a
logged-in account). For provenance, add `id-token: write`. Example workflow
(trigger it manually or on a tag):

```yaml
name: publish-dsh-plugin
on:
  push:
    tags: ['dsh-plugin-v*']
permissions:
  contents: read
  id-token: write
jobs:
  npm:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: 22
          registry-url: https://registry.npmjs.org
      - run: cargo build
      - run: node --test dsh-plugin/test/*.test.mjs
      - run: npm publish --access public --provenance
        working-directory: dsh-plugin
        env:
          NODE_AUTH_TOKEN: ${{ secrets.NPM_TOKEN }}
```

Adjust `working-directory`/`--workspace` to your layout; the package is not part of a
pnpm workspace, so a plain `cd dsh-plugin && npm publish` is equivalent.

## What users need

- A DeepSeek Harness version that supports bundles (`dsh plugin add`).
- `dsh plugin add @wst7/dsh-tam` installs the optional `@wst7/tam` dependency automatically
  when the compatible version is on npm; the launcher downloads the platform binary on
  first use. If the optional dependency is skipped (offline/mirror lag), install tam
  manually or set `command` in the profile patch.
- `tam --json project list` returning `tam.agent/v1` is the smoke test.
- Restart DSH after installing/upgrading; the app does not hot-add bundles.

The plugin README has the consumer install/config/tool reference:
[`dsh-plugin/README.md`](README.md).
