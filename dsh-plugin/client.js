/**
 * dsh-tam browser half.
 *
 * This is a prebuilt DSH client bundle: the shell executes it as a classic
 * script, so it registers a closure factory instead of using ESM imports.
 * React comes from the frozen module table; everything else is plain JS.
 */
window.__ModuleLoader__.load({
  id: 'dsh-tam',
  factory: (require) => {
    const React = require('react');
    const h = React.createElement;
    const { Fragment, useCallback, useEffect, useMemo, useRef, useState } = React;

    const API = '/plugins/dsh-tam/api';
    const STYLE_ID = 'dsh-tam-panel-style';

    const CSS = String.raw`
      .tam-entry {
        display: flex;
        align-items: center;
        gap: 8px;
        width: 100%;
        height: 36px;
        padding: 0 10px;
        border: 0;
        border-radius: 8px;
        background: transparent;
        color: var(--dsw-alias-label-secondary, #5a5a5a);
        font: inherit;
        font-size: 13px;
        line-height: 18px;
        cursor: pointer;
        text-align: left;
      }
      .tam-entry:hover { background: var(--dsw-alias-interactive-bg-hover, rgba(0,0,0,.05)); color: var(--dsw-alias-label-primary, #111); }
      .tam-entry:focus-visible { outline: 2px solid var(--dsw-alias-state-business-primary, #4d6bfe); outline-offset: -2px; }
      .tam-entry-icon { display: inline-flex; flex: none; }
      .tam-entry-label { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }

      .tam-overlay {
        position: fixed;
        inset: 0;
        z-index: 1000;
        display: flex;
        justify-content: flex-end;
        background: var(--dsw-alias-bg-mask-1, rgba(16, 20, 30, .28));
      }
      .tam-panel {
        display: flex;
        flex-direction: column;
        width: min(860px, 94vw);
        height: 100%;
        background: var(--dsw-alias-bg-layer-1, #fff);
        color: var(--dsw-alias-label-primary, #111);
        border-left: 1px solid var(--dsw-alias-border-l2, rgba(0,0,0,.12));
        box-shadow: -18px 0 48px rgba(0,0,0,.16);
        font: inherit;
        font-size: 13px;
        line-height: 18px;
      }
      .tam-head {
        display: flex;
        align-items: center;
        gap: 10px;
        min-height: 52px;
        padding: 0 18px;
        border-bottom: 1px solid var(--dsw-alias-border-l2, rgba(0,0,0,.12));
      }
      .tam-title { font-size: 15px; font-weight: 600; line-height: 22px; }
      .tam-sub { color: var(--dsw-alias-label-tertiary, #888); font-size: 12px; }
      .tam-spacer { flex: 1; }
      .tam-iconbtn {
        display: inline-flex; align-items: center; justify-content: center;
        width: 30px; height: 30px; padding: 0; border: 0; border-radius: 7px;
        background: transparent; color: var(--dsw-alias-label-secondary, #5a5a5a); cursor: pointer;
      }
      .tam-iconbtn:hover { background: var(--dsw-alias-interactive-bg-hover, rgba(0,0,0,.05)); color: var(--dsw-alias-label-primary, #111); }
      .tam-body { display: flex; flex: 1; min-height: 0; }
      .tam-listpane {
        display: flex; flex-direction: column; flex: none; width: 300px; min-width: 0;
        border-right: 1px solid var(--dsw-alias-border-l2, rgba(0,0,0,.12));
      }
      .tam-editorpane { display: flex; flex-direction: column; flex: 1; min-width: 0; overflow: auto; }
      .tam-toolbar { display: flex; gap: 8px; align-items: center; padding: 12px; border-bottom: 1px solid var(--dsw-alias-border-l2, rgba(0,0,0,.12)); }
      .tam-select, .tam-input, .tam-textarea {
        box-sizing: border-box; width: 100%; border: 1px solid var(--dsw-alias-border-l2, rgba(0,0,0,.16));
        border-radius: 7px; background: var(--dsw-alias-bg-layer-1, #fff); color: inherit;
        font: inherit; font-size: 13px; line-height: 18px; padding: 7px 9px; outline: none;
      }
      .tam-select:focus, .tam-input:focus, .tam-textarea:focus { border-color: var(--dsw-alias-state-business-primary, #4d6bfe); }
      .tam-textarea { min-height: 110px; resize: vertical; }
      .tam-toolbar .tam-select { width: auto; min-width: 150px; }
      .tam-btn {
        display: inline-flex; align-items: center; justify-content: center; gap: 6px;
        height: 30px; padding: 0 11px; border: 1px solid var(--dsw-alias-border-l2, rgba(0,0,0,.16));
        border-radius: 7px; background: var(--dsw-alias-bg-layer-1, #fff); color: inherit;
        font: inherit; font-size: 12px; line-height: 16px; cursor: pointer; white-space: nowrap;
      }
      .tam-btn:hover:not(:disabled) { background: var(--dsw-alias-interactive-bg-hover, rgba(0,0,0,.05)); }
      .tam-btn:disabled { opacity: .45; cursor: default; }
      .tam-btn-primary { border-color: var(--dsw-alias-state-business-primary, #4d6bfe); background: var(--dsw-alias-state-business-primary, #4d6bfe); color: #fff; }
      .tam-btn-primary:hover:not(:disabled) { filter: brightness(.95); background: var(--dsw-alias-state-business-primary, #4d6bfe); }
      .tam-btn-danger { color: var(--dsw-alias-state-error-primary, #c62828); }
      .tam-tasklist { flex: 1; min-height: 0; overflow: auto; padding: 6px; }
      .tam-task {
        display: flex; flex-direction: column; gap: 3px; width: 100%; box-sizing: border-box;
        padding: 9px 10px; border: 0; border-radius: 8px; background: transparent;
        color: inherit; font: inherit; text-align: left; cursor: pointer;
      }
      .tam-task:hover { background: var(--dsw-alias-interactive-bg-hover, rgba(0,0,0,.05)); }
      .tam-task[data-selected="true"] { background: var(--dsw-alias-interactive-bg-active, rgba(77,107,254,.10)); }
      .tam-task-title { display: flex; align-items: center; gap: 6px; min-width: 0; font-size: 13px; line-height: 18px; }
      .tam-task-title span:first-child { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
      .tam-task-meta { display: flex; align-items: center; gap: 8px; color: var(--dsw-alias-label-tertiary, #888); font-size: 11px; line-height: 16px; }
      .tam-pill { display: inline-flex; align-items: center; flex: none; height: 18px; padding: 0 6px; border-radius: 999px; font-size: 10px; line-height: 12px; background: var(--dsw-alias-bg-layer-2, rgba(0,0,0,.06)); color: var(--dsw-alias-label-secondary, #5a5a5a); }
      .tam-pill[data-status="in-progress"] { color: var(--dsw-alias-state-business-primary, #4d6bfe); }
      .tam-pill[data-status="done"] { color: var(--dsw-alias-state-success-primary, #2e7d32); }
      .tam-empty { padding: 22px 14px; color: var(--dsw-alias-label-tertiary, #888); text-align: center; font-size: 12px; line-height: 18px; }
      .tam-form { display: flex; flex-direction: column; gap: 12px; padding: 18px; }
      .tam-field { display: flex; flex-direction: column; gap: 5px; }
      .tam-label { color: var(--dsw-alias-label-secondary, #5a5a5a); font-size: 12px; line-height: 16px; }
      .tam-row { display: flex; gap: 12px; }
      .tam-row > .tam-field { flex: 1; min-width: 0; }
      .tam-actions { display: flex; gap: 8px; align-items: center; padding-top: 4px; }
      .tam-error { margin: 0 18px 12px; padding: 9px 11px; border-radius: 7px; background: var(--dsw-alias-bg-layer-2, #fdecea); color: var(--dsw-alias-state-error-primary, #c62828); font-size: 12px; line-height: 18px; white-space: pre-wrap; }
      .tam-note { color: var(--dsw-alias-label-tertiary, #888); font-size: 11px; line-height: 16px; }
      .tam-newproject { display: flex; gap: 6px; padding: 0 12px 10px; }
      .tam-newproject .tam-input { min-width: 0; }
      @media (max-width: 720px) {
        .tam-panel { width: 100%; }
        .tam-body { flex-direction: column; }
        .tam-listpane { width: 100%; max-height: 42%; border-right: 0; border-bottom: 1px solid var(--dsw-alias-border-l2, rgba(0,0,0,.12)); }
      }
    `;

    function ensureStyle() {
      let tag = document.getElementById(STYLE_ID);
      if (tag === null) {
        tag = document.createElement('style');
        tag.id = STYLE_ID;
        tag.textContent = CSS;
        document.head.append(tag);
      }
      return tag;
    }

    async function doRequest(path, init = {}) {
      const base = location.origin === 'null' || location.origin === '' ? 'http://dsh.internal' : location.origin;
      const url = new URL(API + path, base);
      const requestInit = {
        headers: { 'content-type': 'application/json' },
        ...init,
      };
      const transport = globalThis.__DSH_TRANSPORT__;
      const transportFetch = transport && typeof transport.fetch === 'function' ? transport.fetch : null;
      const send = transportFetch ?? ((input, options) => globalThis.fetch(input, options));
      let response;
      try {
        response = await send(url, requestInit);
      } catch (error) {
        if (transportFetch !== null) response = await globalThis.fetch(url, requestInit);
        else throw error;
      }
      const body = await response.json().catch(() => ({}));
      if (!response.ok || body.ok === false) {
        const message = body && body.error && body.error.message
          ? `${body.error.code}: ${body.error.message}`
          : `HTTP ${response.status}`;
        throw new Error(message);
      }
      return body.data;
    }

    const STATUS_OPTIONS = ['todo', 'in-progress', 'done'];
    const STATUS_LABELS = { todo: '待办', 'in-progress': '进行中', done: '完成' };
    const PRIORITY_LABELS = { 1: 'P1 最高', 2: 'P2 高', 3: 'P3 中', 4: 'P4 低', 5: 'P5 最低' };

    function blankDraft() {
      return { id: null, title: '', description: '', status: 'todo', priority: 3, tagsText: '' };
    }

    function tagsOf(text) {
      return text.split(',').map((tag) => tag.trim()).filter((tag) => tag.length > 0);
    }

    function TaskIcon() {
      return h('svg', {
        width: 16,
        height: 16,
        viewBox: '0 0 16 16',
        fill: 'none',
        'aria-hidden': 'true',
      },
        h('rect', { x: 2.5, y: 2.5, width: 11, height: 11, rx: 2.5, stroke: 'currentColor', strokeWidth: 1.3 }),
        h('path', { d: 'M5.2 8.1l1.6 1.6 3.8-3.8', stroke: 'currentColor', strokeWidth: 1.4, strokeLinecap: 'round', strokeLinejoin: 'round' }),
      );
    }

    function CloseIcon() {
      return h('svg', { width: 16, height: 16, viewBox: '0 0 16 16', fill: 'none', 'aria-hidden': 'true' },
        h('path', { d: 'M4 4l8 8M12 4l-8 8', stroke: 'currentColor', strokeWidth: 1.4, strokeLinecap: 'round' }),
      );
    }

    function TamPanel({ onClose }) {
      const [meta, setMeta] = useState({ defaultProject: '', defaultAgent: 'dsh' });
      const [projects, setProjects] = useState([]);
      const [project, setProject] = useState('');
      const [tasks, setTasks] = useState([]);
      const [draft, setDraft] = useState(blankDraft);
      const [loading, setLoading] = useState(true);
      const [saving, setSaving] = useState(false);
      const [error, setError] = useState('');
      const [newProject, setNewProject] = useState('');
      const alive = useRef(true);

      const applyProjects = useCallback((list, preferred) => {
        setProjects(list);
        const next = list.some((item) => item.name === preferred)
          ? preferred
          : (list[0] ? list[0].name : '');
        setProject(next);
        return next;
      }, []);

      const loadTasks = useCallback(async (name, signal) => {
        if (!name) {
          setTasks([]);
          return;
        }
        const data = await doRequest(`/tasks?project=${encodeURIComponent(name)}`, { signal });
        if (!alive.current) return;
        setTasks(Array.isArray(data.tasks) ? data.tasks : []);
      }, []);

      const loadAll = useCallback(async (signal) => {
        setLoading(true);
        setError('');
        try {
          const [nextMeta, list] = await Promise.all([
            doRequest('/meta', { signal }),
            doRequest('/projects', { signal }),
          ]);
          if (!alive.current) return;
          setMeta(nextMeta);
          const next = applyProjects(Array.isArray(list) ? list : [], nextMeta.defaultProject);
          await loadTasks(next, signal);
        } catch (reason) {
          if (alive.current) setError(reason instanceof Error ? reason.message : String(reason));
        } finally {
          if (alive.current) setLoading(false);
        }
      }, [applyProjects, loadTasks]);

      useEffect(() => {
        alive.current = true;
        const controller = new AbortController();
        loadAll(controller.signal);
        return () => {
          alive.current = false;
          controller.abort();
        };
      }, [loadAll]);

      useEffect(() => {
        const onKeyDown = (event) => {
          if (event.key === 'Escape') onClose();
        };
        window.addEventListener('keydown', onKeyDown);
        return () => { window.removeEventListener('keydown', onKeyDown); };
      }, [onClose]);

      const selectProject = async (name) => {
        setProject(name);
        setDraft(blankDraft());
        setError('');
        try {
          await loadTasks(name);
        } catch (reason) {
          setError(reason instanceof Error ? reason.message : String(reason));
        }
      };

      const selectTask = (task) => {
        setDraft({
          id: task.id,
          title: task.title ?? '',
          description: task.description ?? '',
          status: task.status ?? 'todo',
          priority: task.priority ?? 3,
          tagsText: Array.isArray(task.tags) ? task.tags.join(', ') : '',
        });
        setError('');
      };

      const refresh = async () => {
        setError('');
        try {
          const list = await doRequest('/projects');
          const next = applyProjects(Array.isArray(list) ? list : [], project);
          await loadTasks(next);
        } catch (reason) {
          setError(reason instanceof Error ? reason.message : String(reason));
        }
      };

      const save = async () => {
        if (!project) {
          setError('请先选择一个项目');
          return;
        }
        if (!draft.title.trim()) {
          setError('任务标题不能为空');
          return;
        }
        setSaving(true);
        setError('');
        const body = {
          project,
          title: draft.title.trim(),
          description: draft.description,
          status: draft.status,
          priority: Number(draft.priority),
          tags: tagsOf(draft.tagsText),
        };
        try {
          if (draft.id === null) {
            const created = await doRequest('/tasks', { method: 'POST', body: JSON.stringify(body) });
            await loadTasks(project);
            if (created && created.task) selectTask(created.task);
          } else {
            const updated = await doRequest(`/tasks/${draft.id}?project=${encodeURIComponent(project)}`, {
              method: 'PATCH',
              body: JSON.stringify(body),
            });
            await loadTasks(project);
            if (updated && updated.task) selectTask(updated.task);
          }
        } catch (reason) {
          setError(reason instanceof Error ? reason.message : String(reason));
        } finally {
          setSaving(false);
        }
      };

      const remove = async () => {
        if (draft.id === null) return;
        if (!window.confirm(`删除任务 #${draft.id}「${draft.title}」？`)) return;
        setSaving(true);
        setError('');
        try {
          await doRequest(`/tasks/${draft.id}?project=${encodeURIComponent(project)}`, { method: 'DELETE' });
          setDraft(blankDraft());
          await loadTasks(project);
        } catch (reason) {
          setError(reason instanceof Error ? reason.message : String(reason));
        } finally {
          setSaving(false);
        }
      };

      const addProject = async () => {
        const name = newProject.trim();
        if (!name) return;
        setError('');
        try {
          await doRequest('/projects', { method: 'POST', body: JSON.stringify({ name }) });
          setNewProject('');
          const list = await doRequest('/projects');
          const next = applyProjects(Array.isArray(list) ? list : [], name);
          await loadTasks(next);
        } catch (reason) {
          setError(reason instanceof Error ? reason.message : String(reason));
        }
      };

      const priority = Number(draft.priority);
      const list = useMemo(() => tasks, [tasks]);

      return h('div', {
        className: 'tam-overlay',
        onMouseDown: (event) => { if (event.target === event.currentTarget) onClose(); },
      },
        h('div', { className: 'tam-panel', role: 'dialog', 'aria-modal': 'true', 'aria-label': '任务面板' },
          h('div', { className: 'tam-head' },
            h(TaskIcon),
            h('div', null,
              h('div', { className: 'tam-title' }, '任务'),
              h('div', { className: 'tam-sub' }, `${meta.defaultAgent || 'dsh'} · 手动编辑`),
            ),
            h('div', { className: 'tam-spacer' }),
            h('button', { type: 'button', className: 'tam-iconbtn', 'aria-label': '关闭', title: '关闭', onClick: onClose }, h(CloseIcon)),
          ),
          error ? h('div', { className: 'tam-error', role: 'alert' }, error) : null,
          h('div', { className: 'tam-body' },
            h('div', { className: 'tam-listpane' },
              h('div', { className: 'tam-toolbar' },
                h('select', {
                  className: 'tam-select',
                  value: project,
                  disabled: loading || projects.length === 0,
                  'aria-label': '项目',
                  onChange: (event) => { selectProject(event.target.value); },
                }, projects.length === 0
                  ? h('option', { value: '' }, '没有项目')
                  : projects.map((item) => h('option', { key: item.id ?? item.name, value: item.name }, item.name))),
                h('button', { type: 'button', className: 'tam-btn', onClick: refresh, disabled: loading }, '刷新'),
              ),
              h('div', { className: 'tam-newproject' },
                h('input', {
                  className: 'tam-input',
                  value: newProject,
                  placeholder: '新项目名',
                  'aria-label': '新项目名',
                  onChange: (event) => setNewProject(event.target.value),
                  onKeyDown: (event) => { if (event.key === 'Enter') addProject(); },
                }),
                h('button', { type: 'button', className: 'tam-btn', onClick: addProject, disabled: !newProject.trim() }, '新建'),
              ),
              h('div', { className: 'tam-tasklist' },
                loading
                  ? h('div', { className: 'tam-empty' }, '加载中…')
                  : list.length === 0
                    ? h('div', { className: 'tam-empty' }, '这个项目还没有任务')
                    : list.map((task) => h('button', {
                      type: 'button',
                      key: task.id,
                      className: 'tam-task',
                      'data-selected': String(draft.id === task.id),
                      onClick: () => selectTask(task),
                    },
                      h('span', { className: 'tam-task-title' },
                        h('span', null, task.title),
                        h('span', { className: 'tam-pill', 'data-status': task.status }, STATUS_LABELS[task.status] ?? task.status),
                      ),
                      h('span', { className: 'tam-task-meta' },
                        h('span', null, `#${task.id}`),
                        h('span', null, `P${task.priority ?? 3}`),
                        task.assigned_agent ? h('span', null, task.assigned_agent) : null,
                      ),
                    )),
              ),
              h('div', { className: 'tam-toolbar', style: { borderTop: '1px solid var(--dsw-alias-border-l2, rgba(0,0,0,.12))', borderBottom: 0 } },
                h('button', {
                  type: 'button',
                  className: 'tam-btn tam-btn-primary',
                  onClick: () => { setDraft(blankDraft()); setError(''); },
                  disabled: !project || saving,
                }, '新建任务'),
              ),
            ),
            h('div', { className: 'tam-editorpane' },
              h('div', { className: 'tam-form' },
                h('div', { className: 'tam-field' },
                  h('label', { className: 'tam-label' }, draft.id === null ? '新建任务标题' : `任务 #${draft.id} 标题`),
                  h('input', {
                    className: 'tam-input',
                    value: draft.title,
                    placeholder: '任务标题',
                    onChange: (event) => setDraft({ ...draft, title: event.target.value }),
                  }),
                ),
                h('div', { className: 'tam-field' },
                  h('label', { className: 'tam-label' }, '描述 / 上下文'),
                  h('textarea', {
                    className: 'tam-textarea',
                    value: draft.description,
                    placeholder: '描述、验收标准、上下文…',
                    onChange: (event) => setDraft({ ...draft, description: event.target.value }),
                  }),
                ),
                h('div', { className: 'tam-row' },
                  h('div', { className: 'tam-field' },
                    h('label', { className: 'tam-label' }, '状态'),
                    h('select', {
                      className: 'tam-select',
                      value: draft.status,
                      onChange: (event) => setDraft({ ...draft, status: event.target.value }),
                    }, STATUS_OPTIONS.map((value) => h('option', { key: value, value }, STATUS_LABELS[value]))),
                  ),
                  h('div', { className: 'tam-field' },
                    h('label', { className: 'tam-label' }, '优先级'),
                    h('select', {
                      className: 'tam-select',
                      value: priority,
                      onChange: (event) => setDraft({ ...draft, priority: Number(event.target.value) }),
                    }, [1, 2, 3, 4, 5].map((value) => h('option', { key: value, value }, PRIORITY_LABELS[value]))),
                  ),
                ),
                h('div', { className: 'tam-field' },
                  h('label', { className: 'tam-label' }, '标签（逗号分隔）'),
                  h('input', {
                    className: 'tam-input',
                    value: draft.tagsText,
                    placeholder: 'bug, backend, agent',
                    onChange: (event) => setDraft({ ...draft, tagsText: event.target.value }),
                  }),
                ),
                h('div', { className: 'tam-actions' },
                  h('button', { type: 'button', className: 'tam-btn tam-btn-primary', onClick: save, disabled: saving || !project }, saving ? '保存中…' : '保存'),
                  draft.id !== null
                    ? h('button', { type: 'button', className: 'tam-btn tam-btn-danger', onClick: remove, disabled: saving }, '删除')
                    : null,
                  h('div', { className: 'tam-spacer' }),
                  draft.id !== null ? h('span', { className: 'tam-note' }, `ID ${draft.id}`) : null,
                ),
                h('div', { className: 'tam-note' },
                  project
                    ? `当前项目：${project}${meta.defaultProject ? `（默认：${meta.defaultProject}）` : ''}`
                    : '尚无项目；在左侧输入名称新建。',
                ),
              ),
            ),
          ),
        ),
      );
    }

    function TamEntry({ wide }) {
      const [open, setOpen] = useState(false);
      return h(Fragment, null,
        h('button', {
          type: 'button',
          className: 'tam-entry',
          title: '任务面板',
          'aria-label': '打开任务面板',
          onClick: () => setOpen((value) => !value),
        },
          h('span', { className: 'tam-entry-icon' }, h(TaskIcon)),
          wide ? h('span', { className: 'tam-entry-label' }, '任务') : null,
        ),
        open ? h(TamPanel, { onClose: () => setOpen(false) }) : null,
      );
    }

    function apply(ctx) {
      ensureStyle();
      ctx.effect(() => {
        ensureStyle();
        return () => {
          const tag = document.getElementById(STYLE_ID);
          if (tag !== null) tag.remove();
        };
      }, 'dsh-tam: panel styles');
      ctx.slots.inject('sidebar.footer.action', () => ctx.slots.register({
        name: 'sidebar.footer.action',
        id: 'tam-panel',
        order: 20,
        label: 'tam',
      }, TamEntry));
    }

    return { name: 'tam-panel', inject: ['slots'], apply };
  },
});
