import 'monaco-editor/esm/vs/editor/browser/controller/coreCommands';
import 'monaco-editor/esm/vs/editor/browser/widget/codeEditorWidget';
import 'monaco-editor/esm/vs/editor/browser/widget/diffEditorWidget';
import 'monaco-editor/esm/vs/editor/browser/widget/diffNavigator';
import 'monaco-editor/esm/vs/editor/contrib/anchorSelect/anchorSelect';
import 'monaco-editor/esm/vs/editor/contrib/bracketMatching/bracketMatching';
import 'monaco-editor/esm/vs/editor/contrib/caretOperations/caretOperations';
import 'monaco-editor/esm/vs/editor/contrib/caretOperations/transpose';
import 'monaco-editor/esm/vs/editor/contrib/clipboard/clipboard';
import 'monaco-editor/esm/vs/editor/contrib/codeAction/codeActionContributions';
import 'monaco-editor/esm/vs/editor/contrib/codelens/codelensController';
import 'monaco-editor/esm/vs/editor/contrib/colorPicker/colorContributions';
import 'monaco-editor/esm/vs/editor/contrib/comment/comment';
import 'monaco-editor/esm/vs/editor/contrib/contextmenu/contextmenu';
import 'monaco-editor/esm/vs/editor/contrib/cursorUndo/cursorUndo';
import 'monaco-editor/esm/vs/editor/contrib/dnd/dnd';
import 'monaco-editor/esm/vs/editor/contrib/documentSymbols/documentSymbols';
import 'monaco-editor/esm/vs/editor/contrib/find/findController';
import 'monaco-editor/esm/vs/editor/contrib/folding/folding';
import 'monaco-editor/esm/vs/editor/contrib/fontZoom/fontZoom';
import 'monaco-editor/esm/vs/editor/contrib/format/formatActions';
import 'monaco-editor/esm/vs/editor/contrib/gotoError/gotoError';
import 'monaco-editor/esm/vs/editor/contrib/gotoSymbol/goToCommands';
import 'monaco-editor/esm/vs/editor/contrib/gotoSymbol/link/goToDefinitionAtPosition';
import 'monaco-editor/esm/vs/editor/contrib/hover/hover';
import 'monaco-editor/esm/vs/editor/contrib/inPlaceReplace/inPlaceReplace';
import 'monaco-editor/esm/vs/editor/contrib/indentation/indentation';
import 'monaco-editor/esm/vs/editor/contrib/inlayHints/inlayHintsController';
import 'monaco-editor/esm/vs/editor/contrib/linesOperations/linesOperations';
import 'monaco-editor/esm/vs/editor/contrib/linkedEditing/linkedEditing';
import 'monaco-editor/esm/vs/editor/contrib/links/links';
import 'monaco-editor/esm/vs/editor/contrib/multicursor/multicursor';
import 'monaco-editor/esm/vs/editor/contrib/parameterHints/parameterHints';
import 'monaco-editor/esm/vs/editor/contrib/rename/rename';
import 'monaco-editor/esm/vs/editor/contrib/smartSelect/smartSelect';
import 'monaco-editor/esm/vs/editor/contrib/snippet/snippetController2';
import 'monaco-editor/esm/vs/editor/contrib/suggest/suggestController';
import 'monaco-editor/esm/vs/editor/contrib/toggleTabFocusMode/toggleTabFocusMode';
import 'monaco-editor/esm/vs/editor/contrib/unusualLineTerminators/unusualLineTerminators';
import 'monaco-editor/esm/vs/editor/contrib/viewportSemanticTokens/viewportSemanticTokens';
import 'monaco-editor/esm/vs/editor/contrib/wordHighlighter/wordHighlighter';
import 'monaco-editor/esm/vs/editor/contrib/wordOperations/wordOperations';
import 'monaco-editor/esm/vs/editor/contrib/wordPartOperations/wordPartOperations';
import 'monaco-editor/esm/vs/editor/standalone/browser/accessibilityHelp/accessibilityHelp';
import 'monaco-editor/esm/vs/editor/standalone/browser/iPadShowKeyboard/iPadShowKeyboard';
import 'monaco-editor/esm/vs/editor/standalone/browser/inspectTokens/inspectTokens';
import 'monaco-editor/esm/vs/editor/standalone/browser/quickAccess/standaloneCommandsQuickAccess';
import 'monaco-editor/esm/vs/editor/standalone/browser/quickAccess/standaloneGotoLineQuickAccess';
import 'monaco-editor/esm/vs/editor/standalone/browser/quickAccess/standaloneGotoSymbolQuickAccess';
import 'monaco-editor/esm/vs/editor/standalone/browser/quickAccess/standaloneHelpQuickAccess';
import 'monaco-editor/esm/vs/editor/standalone/browser/referenceSearch/standaloneReferenceSearch';
import 'monaco-editor/esm/vs/editor/standalone/browser/toggleHighContrast/toggleHighContrast';

import * as monaco from 'monaco-editor/esm/vs/editor/editor.api';
import exampleCode from './example-code.rs';

import './index.css';
import { conf, grammar } from './rust-grammar';
import fake_std from './fake_std.rs';
import fake_core from './fake_core.rs';
import fake_alloc from './fake_alloc.rs';

var state;
var allTokens;

self.MonacoEnvironment = {
    getWorkerUrl: () => './editor.worker.bundle.js',
};

const modeId = 'rust';
monaco.languages.register({
    id: modeId,
});

const delay = (ms) => new Promise((res) => setTimeout(res, ms));

monaco.languages.onLanguage(modeId, async () => {
    console.log(modeId);

    monaco.languages.setLanguageConfiguration(modeId, conf);
    monaco.languages.setMonarchTokensProvider(modeId, grammar);
});

const registerRA = async () => {
    monaco.languages.registerHoverProvider(modeId, {
        provideHover: (_, pos) => state.hover(pos.lineNumber, pos.column),
    });
    monaco.languages.registerCodeLensProvider(modeId, {
        async provideCodeLenses(m) {
            const code_lenses = await state.code_lenses();
            const lenses = code_lenses.map(({ range, command }) => {
                const position = {
                    column: range.startColumn,
                    lineNumber: range.startLineNumber,
                };

                const references = command.positions.map((pos) => ({ range: pos, uri: m.uri }));
                return {
                    range,
                    command: {
                        id: command.id,
                        title: command.title,
                        arguments: [
                            m.uri,
                            position,
                            references,
                        ],
                    },
                };
            });

            return { lenses, dispose() { } };
        },
    });
    monaco.languages.registerReferenceProvider(modeId, {
        async provideReferences(m, pos, { includeDeclaration }) {
            const references = await state.references(pos.lineNumber, pos.column, includeDeclaration);
            if (references) {
                return references.map(({ range }) => ({ uri: m.uri, range }));
            }
        },
    });
    monaco.languages.registerInlayHintsProvider(modeId, {
        async provideInlayHints(model, range, token) {
            let hints = await state.inlay_hints();
            return hints.map((hint) => {
                if (hint.hint_type == 1) {
                    return {
                        kind: 1,
                        position: { column: hint.range.endColumn, lineNumber: hint.range.endLineNumber },
                        text: `: ${hint.label}`,
                    };
                }
                if (hint.hint_type == 2) {
                    return {
                        kind: 2,
                        position: { column: hint.range.startColumn, lineNumber: hint.range.startLineNumber },
                        text: `${hint.label}:`,
                        whitespaceAfter: true,
                    };
                }
            })
        }
    });
    monaco.languages.registerDocumentHighlightProvider(modeId, {
        async provideDocumentHighlights(_, pos) {
            return await state.references(pos.lineNumber, pos.column, true);
        }
    });
    monaco.languages.registerRenameProvider(modeId, {
        async provideRenameEdits(m, pos, newName) {
            const edits = await state.rename(pos.lineNumber, pos.column, newName);
            if (edits) {
                return {
                    edits: edits.map(edit => ({
                        resource: m.uri,
                        edit,
                    })),
                };
            }
        },
        async resolveRenameLocation(_, pos) {
            return state.prepare_rename(pos.lineNumber, pos.column);
        }
    });
    monaco.languages.registerCompletionItemProvider(modeId, {
        triggerCharacters: [".", ":", "="],
        async provideCompletionItems(_m, pos) {
            const suggestions = await state.completions(pos.lineNumber, pos.column);
            if (suggestions) {
                return { suggestions };
            }
        },
    });
    monaco.languages.registerSignatureHelpProvider(modeId, {
        signatureHelpTriggerCharacters: ['(', ','],
        async provideSignatureHelp(_m, pos) {
            const value = await state.signature_help(pos.lineNumber, pos.column);
            if (!value) return null;
            return {
                value,
                dispose() { },
            };
        },
    });
    monaco.languages.registerDefinitionProvider(modeId, {
        async provideDefinition(m, pos) {
            const list = await state.definition(pos.lineNumber, pos.column);
            if (list) {
                return list.map(def => ({ ...def, uri: m.uri }));
            }
        },
    });
    monaco.languages.registerTypeDefinitionProvider(modeId, {
        async provideTypeDefinition(m, pos) {
            const list = await state.type_definition(pos.lineNumber, pos.column);
            if (list) {
                return list.map(def => ({ ...def, uri: m.uri }));
            }
        },
    });
    monaco.languages.registerImplementationProvider(modeId, {
        async provideImplementation(m, pos) {
            const list = await state.goto_implementation(pos.lineNumber, pos.column);
            if (list) {
                return list.map(def => ({ ...def, uri: m.uri }));
            }
        },
    });
    monaco.languages.registerDocumentSymbolProvider(modeId, {
        async provideDocumentSymbols() {
            return await state.document_symbols();
        }
    });
    monaco.languages.registerOnTypeFormattingEditProvider(modeId, {
        autoFormatTriggerCharacters: [".", "="],
        async provideOnTypeFormattingEdits(_, pos, ch) {
            return await state.type_formatting(pos.lineNumber, pos.column, ch);
        }
    });
    monaco.languages.registerFoldingRangeProvider(modeId, {
        async provideFoldingRanges() {
            return await state.folding_ranges();
        }
    });

    class TokenState {
        constructor(line = 0) {
            this.line = line;
            this.equals = () => true;
        }

        clone() {
            const res = new TokenState(this.line);
            res.line += 1;
            return res;
        }
    }

    function fixTag(tag) {
        switch (tag) {
            case 'builtin': return 'variable.predefined';
            case 'attribute': return 'key';
            case 'macro': return 'number.hex';
            case 'literal': return 'number';
            default: return tag;
        }
    }

    /*monaco.languages.setTokensProvider(modeId, {
        getInitialState: () => new TokenState(),
        tokenize(_, st) {
            const filteredTokens = allTokens
                .filter((token) => token.range.startLineNumber === st.line);

            const tokens = filteredTokens.map((token) => ({
                startIndex: token.range.startColumn - 1,
                scopes: fixTag(token.tag),
            }));
            tokens.sort((a, b) => a.startIndex - b.startIndex);

            return {
                tokens,
                endState: new TokenState(st.line + 1),
            };
        },
    });*/
};


// Create an RA Web worker
const createRA = async () => {
    const worker = new Worker(new URL('./ra-worker.js', import.meta.url));
    const pendingResolve = {};

    let id = 1;
    let ready;

    const callWorker = async (which, ...args) => {
        return new Promise((resolve, _) => {
            pendingResolve[id] = resolve;
            worker.postMessage({
                "which": which,
                "args": args,
                "id": id
            });
            id += 1;
        });
    }

    const proxyHandler = {
        get: (target, prop, _receiver) => {
            if (prop == "then") {
                return Reflect.get(target, prop, _receiver);
            }
            return async (...args) => {
                return callWorker(prop, ...args);
            }
        }
    }

    worker.onmessage = (e) => {
        if (e.data.id == "ra-worker-ready") {
            ready(new Proxy({}, proxyHandler));
            return;
        }
        const pending = pendingResolve[e.data.id];
        if (pending) {
            pending(e.data.result);
            delete pendingResolve[e.data.id];
        }
    }

    return new Promise((resolve, _) => {
        ready = resolve;
    });
}

const start = async () => {
    var loadingText = document.createTextNode("Loading wasm...");
    document.body.appendChild(loadingText);    
    
    let model = monaco.editor.createModel(exampleCode, modeId);
    window.editor = monaco.editor;
    state = null; //await createRA();

    async function update() {
        const res = await state.update(model.getValue());
        monaco.editor.setModelMarkers(model, modeId, res.diagnostics);
        allTokens = res.highlights;
    }

    monaco.editor.defineTheme('vscode-dark-plus', {
        base: 'vs-dark', 
        inherit: true,
        colors: {
            'editorInlayHint.foreground': '#A0A0A0F0',
            'editorInlayHint.background': '#11223300',
        },
        rules: [
          { token: 'keyword.control', foreground: 'C586C0' },
          { token: 'variable', foreground: '9CDCFE' },
          { token: 'support.function', foreground: 'DCDCAA' },
        ],
    });
    document.body.removeChild(loadingText);
    const initRA = async () => {
        state = await createRA();
        await registerRA();
        await state.init(model.getValue(), fake_std, fake_core, fake_alloc);
        await update();
        model.onDidChangeContent(update);
    };
    initRA();
    const myEditor = monaco.editor.create(document.body, {
        theme: 'vscode-dark-plus',
        model: model
    });

    window.onresize = () => myEditor.layout();
};

start().then(() => {
    console.log("start");
})

