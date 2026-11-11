// Splits an assistant message into prose and A2UI 1.0 blocks. The Rust agent
// compiles the model's Express output and sends each block as
// `<a2ui-json>[...messages...]</a2ui-json>` inside the message text.
// Kept free of @a2ui imports so it is safe during server rendering.

export type ContentSegment =
  | { kind: "text"; text: string }
  | { kind: "a2ui"; messages: unknown[] }
  | { kind: "pending" };

const OPEN = "<a2ui-json>";
const CLOSE = "</a2ui-json>";

export function splitA2uiContent(text: string): ContentSegment[] {
  const out: ContentSegment[] = [];
  let i = 0;
  while (i < text.length) {
    const start = text.indexOf(OPEN, i);
    if (start === -1) {
      out.push({ kind: "text", text: text.slice(i) });
      break;
    }
    if (start > i) out.push({ kind: "text", text: text.slice(i, start) });
    const end = text.indexOf(CLOSE, start + OPEN.length);
    if (end === -1) {
      // Still streaming: hide the partial JSON until the block closes.
      out.push({ kind: "pending" });
      break;
    }
    try {
      const parsed = JSON.parse(text.slice(start + OPEN.length, end));
      if (Array.isArray(parsed) && parsed.length > 0) out.push({ kind: "a2ui", messages: parsed });
    } catch {
      out.push({ kind: "text", text: "(The view could not be read.)" });
    }
    i = end + CLOSE.length;
  }
  return out.filter((s) => s.kind !== "text" || s.text.trim() !== "");
}
