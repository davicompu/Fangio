import { useEffect, useMemo, useRef } from "react";
import { useEditor, EditorContent } from "@tiptap/react";
import StarterKit from "@tiptap/starter-kit";
import Link from "@tiptap/extension-link";
import Placeholder from "@tiptap/extension-placeholder";
import Typography from "@tiptap/extension-typography";
import { marked } from "marked";
import TurndownService from "turndown";
import {
  Bold,
  Heading1,
  Heading2,
  Heading3,
  Italic,
  Link as LinkIcon,
  List,
  ListOrdered,
  Quote as QuoteIcon,
  Redo2,
  Undo2,
} from "lucide-react";
import { cn } from "../lib/utils";

interface DraftEditorProps {
  markdown: string;
  onChange: (markdown: string) => void;
  placeholder?: string;
}

/**
 * TipTap-based rich-text editor with a markdown round-trip. Markdown comes in,
 * is rendered to HTML by `marked`, mounted in TipTap, and serialized back to
 * markdown by `turndown` whenever the document changes.
 */
export function DraftEditor({ markdown, onChange, placeholder }: DraftEditorProps) {
  const turndown = useMemo(() => {
    const t = new TurndownService({
      headingStyle: "atx",
      codeBlockStyle: "fenced",
      bulletListMarker: "-",
    });
    // Preserve [TO COMPLETE] markers verbatim
    return t;
  }, []);

  const initialHtml = useMemo(() => marked.parse(markdown || "") as string, []);

  const editor = useEditor({
    extensions: [
      StarterKit,
      Typography,
      Link.configure({ openOnClick: false }),
      Placeholder.configure({
        placeholder: placeholder ?? "Start writing…",
      }),
    ],
    content: initialHtml,
    editorProps: {
      attributes: {
        class:
          "tiptap-editor prose-editorial focus:outline-none p-8 min-h-[60vh] max-w-none",
      },
    },
    onUpdate({ editor }) {
      const html = editor.getHTML();
      const md = turndown.turndown(html);
      onChange(md);
    },
  });

  // When the markdown prop changes from outside (e.g., regenerate), reload the editor
  const lastSeen = useRef(markdown);
  useEffect(() => {
    if (!editor) return;
    if (markdown === lastSeen.current) return;
    lastSeen.current = markdown;
    const html = marked.parse(markdown || "") as string;
    editor.commands.setContent(html, false);
  }, [markdown, editor]);

  useEffect(() => () => editor?.destroy(), [editor]);

  if (!editor) return null;

  return (
    <div className="border border-[var(--color-paper-300)] rounded-[var(--radius-md)] bg-white overflow-hidden shadow-[var(--shadow-paper)]">
      <Toolbar editor={editor} />
      <div className="border-t border-[var(--color-paper-200)]">
        <EditorContent editor={editor} />
      </div>
      <style>{`
        .tiptap-editor h1 { font-family: var(--font-sans); font-size: 26px; font-weight: 600; letter-spacing: -0.02em; margin-top: 1.4em; margin-bottom: 0.6em; color: var(--color-ink-900); }
        .tiptap-editor h2 { font-family: var(--font-sans); font-size: 20px; font-weight: 600; letter-spacing: -0.015em; margin-top: 1.4em; margin-bottom: 0.5em; color: var(--color-ink-900); }
        .tiptap-editor h3 { font-family: var(--font-sans); font-size: 16px; font-weight: 600; letter-spacing: -0.01em; margin-top: 1.3em; margin-bottom: 0.4em; color: var(--color-ink-900); }
        .tiptap-editor p { margin: 0.7em 0; }
        .tiptap-editor ul, .tiptap-editor ol { margin: 0.7em 0; padding-left: 1.6em; }
        .tiptap-editor li { margin: 0.25em 0; }
        .tiptap-editor blockquote { border-left: 3px solid var(--color-paper-300); margin: 1em 0; padding: 0.2em 0 0.2em 1em; color: var(--color-paper-700); font-style: italic; }
        .tiptap-editor code { background: var(--color-paper-100); padding: 0.1em 0.35em; border-radius: 3px; font-family: var(--font-mono); font-size: 0.9em; }
        .tiptap-editor a { color: var(--color-ink-600); text-decoration: underline; text-underline-offset: 2px; }
        .tiptap-editor table { border-collapse: collapse; margin: 1em 0; font-family: var(--font-sans); font-size: 13px; }
        .tiptap-editor th, .tiptap-editor td { border: 1px solid var(--color-paper-300); padding: 0.4em 0.7em; }
        .tiptap-editor th { background: var(--color-paper-100); font-weight: 600; }
        .tiptap-editor p.is-editor-empty:first-child::before { content: attr(data-placeholder); color: var(--color-paper-400); float: left; pointer-events: none; height: 0; }
      `}</style>
    </div>
  );
}

function Toolbar({ editor }: { editor: any }) {
  return (
    <div className="px-3 h-11 flex items-center gap-0.5 bg-[var(--color-paper-100)]">
      <ToolBtn
        active={editor.isActive("heading", { level: 1 })}
        onClick={() => editor.chain().focus().toggleHeading({ level: 1 }).run()}
        title="Heading 1"
      >
        <Heading1 size={14} strokeWidth={1.75} />
      </ToolBtn>
      <ToolBtn
        active={editor.isActive("heading", { level: 2 })}
        onClick={() => editor.chain().focus().toggleHeading({ level: 2 }).run()}
        title="Heading 2"
      >
        <Heading2 size={14} strokeWidth={1.75} />
      </ToolBtn>
      <ToolBtn
        active={editor.isActive("heading", { level: 3 })}
        onClick={() => editor.chain().focus().toggleHeading({ level: 3 }).run()}
        title="Heading 3"
      >
        <Heading3 size={14} strokeWidth={1.75} />
      </ToolBtn>
      <Sep />
      <ToolBtn
        active={editor.isActive("bold")}
        onClick={() => editor.chain().focus().toggleBold().run()}
        title="Bold"
      >
        <Bold size={14} strokeWidth={2} />
      </ToolBtn>
      <ToolBtn
        active={editor.isActive("italic")}
        onClick={() => editor.chain().focus().toggleItalic().run()}
        title="Italic"
      >
        <Italic size={14} strokeWidth={1.75} />
      </ToolBtn>
      <Sep />
      <ToolBtn
        active={editor.isActive("bulletList")}
        onClick={() => editor.chain().focus().toggleBulletList().run()}
        title="Bullet list"
      >
        <List size={14} strokeWidth={1.75} />
      </ToolBtn>
      <ToolBtn
        active={editor.isActive("orderedList")}
        onClick={() => editor.chain().focus().toggleOrderedList().run()}
        title="Numbered list"
      >
        <ListOrdered size={14} strokeWidth={1.75} />
      </ToolBtn>
      <ToolBtn
        active={editor.isActive("blockquote")}
        onClick={() => editor.chain().focus().toggleBlockquote().run()}
        title="Quote"
      >
        <QuoteIcon size={14} strokeWidth={1.75} />
      </ToolBtn>
      <Sep />
      <ToolBtn
        active={editor.isActive("link")}
        onClick={() => {
          const url = window.prompt("Link URL");
          if (url) editor.chain().focus().setLink({ href: url }).run();
          else editor.chain().focus().unsetLink().run();
        }}
        title="Link"
      >
        <LinkIcon size={14} strokeWidth={1.75} />
      </ToolBtn>
      <div className="flex-1" />
      <ToolBtn onClick={() => editor.chain().focus().undo().run()} title="Undo">
        <Undo2 size={14} strokeWidth={1.75} />
      </ToolBtn>
      <ToolBtn onClick={() => editor.chain().focus().redo().run()} title="Redo">
        <Redo2 size={14} strokeWidth={1.75} />
      </ToolBtn>
    </div>
  );
}

function ToolBtn({
  active,
  onClick,
  title,
  children,
}: {
  active?: boolean;
  onClick: () => void;
  title: string;
  children: React.ReactNode;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      title={title}
      className={cn(
        "w-8 h-8 rounded-[var(--radius-xs)] flex items-center justify-center transition-paper",
        active
          ? "bg-[var(--color-ink-700)] text-[var(--color-paper-50)]"
          : "text-[var(--color-paper-600)] hover:bg-[var(--color-paper-200)] hover:text-[var(--color-ink-800)]",
      )}
    >
      {children}
    </button>
  );
}

function Sep() {
  return <span className="w-px h-4 bg-[var(--color-paper-300)] mx-1" />;
}
