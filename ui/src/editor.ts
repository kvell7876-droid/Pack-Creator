/**
 * The code editor (Monaco). Three things are added to it: JSON Schemas per file, so it completes and explains names from what the game has; the Rust checks' problems
 * as squiggles; and their repairs as quick fixes (the lightbulb).
 */
import * as monaco from 'monaco-editor/esm/vs/editor/editor.api';
import 'monaco-editor/esm/vs/editor/editor.all.js';
import 'monaco-editor/esm/vs/language/json/monaco.contribution';
import editorWorker from 'monaco-editor/esm/vs/editor/editor.worker?worker';
import jsonWorker from 'monaco-editor/esm/vs/language/json/json.worker?worker';
import type { Diagnostic } from './types';

(self as unknown as { MonacoEnvironment: unknown }).MonacoEnvironment = {
  getWorker: (_id: string, label: string) => (label === 'json' ? new jsonWorker() : new editorWorker())
};

export { monaco };

/* In development the editor is reachable from the console, to look at its models and markers. */
if (import.meta.env.DEV) (window as unknown as { __monaco: typeof monaco }).__monaco = monaco;

const OWNER = 'pack';

monaco.editor.defineTheme('ph-dark', {
  base: 'vs-dark',
  inherit: true,
  rules: [
    { token: 'string.key.json', foreground: 'd9a8e8' },
    { token: 'string.value.json', foreground: '9fd3a0' },
    { token: 'number', foreground: '7cc4f4' },
    { token: 'keyword.json', foreground: 'f2a65a' }
  ],
  colors: {
    'editor.background': '#1b1c1f',
    'editor.lineHighlightBackground': '#25262a',
    'editorGutter.background': '#1b1c1f',
    'editorLineNumber.foreground': '#555963',
    'editorLineNumber.activeForeground': '#9aa0ab',
    'editor.selectionBackground': '#38405f',
    'editorWidget.background': '#2b2d31',
    'editorSuggestWidget.background': '#2b2d31',
    'editorSuggestWidget.selectedBackground': '#38405f',
    'scrollbarSlider.background': '#44474e80'
  }
});

monaco.languages.json.jsonDefaults.setDiagnosticsOptions({ validate: false, allowComments: false, enableSchemaRequest: false, schemas: [] });

/* ---------- Schemas ---------- */

const schemas = new Map<string, { uri: string; fileMatch: string[]; schema: object }>();

function pushSchemas(): void {
  monaco.languages.json.jsonDefaults.setDiagnosticsOptions({ validate: false, allowComments: false, enableSchemaRequest: false, schemas: [...schemas.values()] });
}

/** Tells the editor which schema a file is written to. The schema only drives completion and hover help: problems come from the Rust checks. */
export function setSchema(rel: string, schema: object | null): void {
  const uri = uriOf(rel).toString();

  if (schema) schemas.set(rel, { uri: 'schema://' + rel, fileMatch: [uri], schema });
  else schemas.delete(rel);

  pushSchemas();
}

export function forgetSchema(rel: string): void {
  if (schemas.delete(rel)) pushSchemas();
}

/* ---------- Models ---------- */

export function uriOf(rel: string): monaco.Uri {
  return monaco.Uri.from({ scheme: 'pack', path: '/' + rel });
}

function languageOf(rel: string): string {
  return rel.endsWith('.json') || rel.endsWith('.mcmeta') ? 'json' : 'plaintext';
}

/** The model of a file, made when it is first opened. */
export function modelFor(rel: string, text: string): monaco.editor.ITextModel {
  const uri = uriOf(rel);

  return monaco.editor.getModel(uri) ?? monaco.editor.createModel(text, languageOf(rel), uri);
}

export function dropModel(rel: string): void {
  monaco.editor.getModel(uriOf(rel))?.dispose();
}

export function renameModel(from: string): void {
  /* A renamed file is closed and opened again under its new name, so its old model is dropped. */
  dropModel(from);
  forgetSchema(from);
}

/* ---------- Problems and their repairs ---------- */

const fixes = new Map<string, { title: string; replacement: string }>();

const keyOf = (uri: string, line: number, column: number, code: string): string => `${uri}|${line}:${column}|${code}`;

const severityOf = { error: monaco.MarkerSeverity.Error, warning: monaco.MarkerSeverity.Warning, info: monaco.MarkerSeverity.Info };

/** Shows a file's problems as squiggles. */
export function showProblems(rel: string, problems: Diagnostic[]): void {
  const model = monaco.editor.getModel(uriOf(rel));

  if (!model) return;

  const uri = model.uri.toString();

  for (const key of [...fixes.keys()]) if (key.startsWith(uri + '|')) fixes.delete(key);

  monaco.editor.setModelMarkers(model, OWNER, problems.map(p => {
    if (p.fix) fixes.set(keyOf(uri, p.line, p.column, p.code), p.fix);

    return {
      severity: severityOf[p.severity],
      message: p.message,
      code: p.code,
      source: 'pack',
      startLineNumber: p.line,
      startColumn: p.column,
      endLineNumber: p.end_line,
      endColumn: p.end_column
    };
  }));
}

monaco.languages.registerCodeActionProvider('json', {
  provideCodeActions(model, _range, context) {
    const actions: monaco.languages.CodeAction[] = [];

    for (const marker of context.markers) {
      const code = typeof marker.code === 'string' ? marker.code : marker.code?.value ?? '';
      const fix = fixes.get(keyOf(model.uri.toString(), marker.startLineNumber, marker.startColumn, code));

      if (!fix) continue;

      actions.push({
        title: fix.title,
        kind: 'quickfix',
        isPreferred: true,
        diagnostics: [marker],
        edit: {
          edits: [{
            resource: model.uri,
            versionId: model.getVersionId(),
            textEdit: { range: new monaco.Range(marker.startLineNumber, marker.startColumn, marker.endLineNumber, marker.endColumn), text: fix.replacement }
          }]
        }
      });
    }

    return { actions, dispose() {} };
  }
});

export function create(host: HTMLElement): monaco.editor.IStandaloneCodeEditor {
  return monaco.editor.create(host, {
    theme: 'ph-dark',
    automaticLayout: true,
    fontFamily: "'Cascadia Code', Consolas, monospace",
    fontSize: 13,
    minimap: { enabled: true, scale: 1 },
    scrollBeyondLastLine: false,
    renderWhitespace: 'selection',
    tabSize: 2,
    insertSpaces: true,
    quickSuggestions: { strings: true, other: true, comments: false },
    suggest: { showWords: false },
    bracketPairColorization: { enabled: true },
    guides: { indentation: true },
    smoothScrolling: true,
    padding: { top: 8 }
  });
}
