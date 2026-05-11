import { StreamLanguage, StringStream } from "@codemirror/language";

/**
 * Lightweight stream-mode highlighter for SpiceDB schema. This does not parse
 * the full grammar — it just classifies tokens for color. Replace with a
 * Lezer grammar later if we want better folding/indent.
 */
const KEYWORDS = new Set([
  "definition",
  "caveat",
  "permission",
  "relation",
  "use",
]);

const TYPE_KEYWORDS = new Set(["nil", "any"]);

interface State {
  inBlockComment: boolean;
}

function token(stream: StringStream, state: State): string | null {
  if (state.inBlockComment) {
    if (stream.match(/.*?\*\//)) {
      state.inBlockComment = false;
    } else {
      stream.skipToEnd();
    }
    return "comment";
  }

  if (stream.match("/*")) {
    state.inBlockComment = true;
    return "comment";
  }

  if (stream.match("//")) {
    stream.skipToEnd();
    return "comment";
  }

  if (stream.eatSpace()) return null;

  if (stream.match(/^[A-Za-z_][A-Za-z0-9_]*/)) {
    const word = stream.current();
    if (KEYWORDS.has(word)) return "keyword";
    if (TYPE_KEYWORDS.has(word)) return "atom";
    return "variableName";
  }

  if (stream.match(/^[+\-*/=<>!&|]+/)) return "operator";
  if (stream.match(/^[{}()\[\],;]/)) return "punctuation";
  if (stream.match(/^[#:]+/)) return "punctuation";

  stream.next();
  return null;
}

export function spiceDbSchemaLanguage() {
  return StreamLanguage.define<State>({
    name: "spicedb-schema",
    startState: () => ({ inBlockComment: false }),
    token,
    languageData: {
      commentTokens: { line: "//", block: { open: "/*", close: "*/" } },
    },
  });
}
