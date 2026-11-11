"use client";

import { ExchangeRateCard } from "@/components/currency";
import {
  useFrontendTool,
  useRenderTool,
  CopilotSidebar,
  CopilotChatMessageView,
  CopilotChatAssistantMessage,
} from "@copilotkit/react-core/v2";
import dynamic from "next/dynamic";
import { splitA2uiContent } from "@/components/a2uiContent";
import React, { useState, useEffect } from "react";
import { z } from "zod";

// @a2ui/react touches browser globals at import time, so load it client-only.
const A2uiMessages = dynamic(
  () => import("@/components/A2uiMessages").then((m) => m.A2uiMessages),
  { ssr: false },
);

const DebugContext = React.createContext<{ debug: boolean; setDebug: (d: boolean) => void }>({
  debug: false,
  setDebug: () => {},
});

const CustomMarkdownRenderer = ({ content, ...props }: any) => {
  const { debug } = React.useContext(DebugContext);
  const segments = React.useMemo(() => splitA2uiContent(content || ""), [content]);
  const blocks = segments.filter((s) => s.kind === "a2ui") as { kind: "a2ui"; messages: unknown[] }[];
  const lastKey = blocks.length ? JSON.stringify(blocks[blocks.length - 1].messages) : "";

  // Mirror the newest view into the main dashboard viewport.
  useEffect(() => {
    if (!lastKey) return;
    window.dispatchEvent(
      new CustomEvent("a2ui-surface-active", { detail: { messages: JSON.parse(lastKey) } }),
    );
  }, [lastKey]);

  return (
    <div className="flex flex-col gap-2 w-full">
      {segments.map((seg, idx) =>
        seg.kind === "text" ? (
          <CopilotChatAssistantMessage.MarkdownRenderer key={idx} {...props} content={seg.text} />
        ) : seg.kind === "a2ui" ? (
          <A2uiMessages key={idx} messages={seg.messages} />
        ) : (
          <p key={idx} className="text-xs text-slate-400">Building the view…</p>
        ),
      )}
      {debug && blocks.length > 0 && (
        <div className="mt-2 p-3 bg-slate-950/80 border border-slate-800 rounded-xl font-mono text-[10px] text-emerald-400 max-h-[250px] overflow-auto text-left select-all">
          <div className="text-[9px] uppercase tracking-wider text-slate-500 font-bold mb-1 border-b border-slate-900 pb-1">
            A2UI 1.0 messages (compiled from Express by the Rust agent)
          </div>
          <pre>{JSON.stringify(blocks.map((b) => b.messages), null, 2)}</pre>
        </div>
      )}
    </div>
  );
};

const CustomAssistantMessage = (props: any) => {
  return (
    <CopilotChatAssistantMessage
      {...props}
      markdownRenderer={CustomMarkdownRenderer}
    />
  );
};

const CustomMessageView = (props: any) => {
  return (
    <CopilotChatMessageView
      {...props}
      assistantMessage={CustomAssistantMessage}
    />
  );
};

const Sidebar = CopilotSidebar as any;

export default function CopilotKitPage() {
  const [themeColor, setThemeColor] = useState("#10b981"); // Mint green primary theme
  const [debug, setDebug] = useState(false);

  useEffect(() => {
    const saved = localStorage.getItem("a2ui_debug") === "true";
    setDebug(saved);
  }, []);

  const handleSetDebug = (val: boolean) => {
    setDebug(val);
    localStorage.setItem("a2ui_debug", String(val));
  };

  useEffect(() => {
    const handleAction = (e: any) => {
      const action = e.detail;
      console.log("React A2UI Action received:", action);
      if (action && action.name) {
        let msg = "";
        if (action.name === 'convert') {
          msg = `Convert ${action.params?.amount || 1} ${action.params?.from || 'USD'} to ${action.params?.to || 'EUR'}`;
        } else if (action.name === 'show_trends') {
          msg = `Show exchange rate trends for ${action.params?.from || 'USD'} to ${action.params?.to || 'EUR'}`;
        } else {
          msg = `Run action: ${action.name}`;
        }

        const textarea = document.querySelector("textarea") as HTMLTextAreaElement;
        if (textarea) {
          textarea.value = msg;
          textarea.dispatchEvent(new Event("input", { bubbles: true }));
          setTimeout(() => {
            const submitBtn = document.querySelector("button[type='submit']") as HTMLButtonElement;
            if (submitBtn) {
              submitBtn.click();
            } else {
              const enterEvent = new KeyboardEvent("keydown", {
                key: "Enter",
                code: "Enter",
                keyCode: 13,
                which: 13,
                bubbles: true
              });
              textarea.dispatchEvent(enterEvent);
            }
          }, 150);
        }
      }
    };
    window.addEventListener("a2ui-action", handleAction);
    return () => window.removeEventListener("a2ui-action", handleAction);
  }, []);

  // 🪁 Frontend Actions
  useFrontendTool({
    name: "setThemeColor",
    description: "Set the theme color of the workspace.",
    parameters: z.object({
      themeColor: z.string().describe("The theme color to set. Make sure to pick nice colors."),
    }),
    async handler({ themeColor }) {
      setThemeColor(themeColor);
    },
  });

  return (
    <DebugContext.Provider value={{ debug, setDebug: handleSetDebug }}>
      <main
        style={
          { "--copilot-kit-primary-color": themeColor } as React.CSSProperties
        }
      >
        <Sidebar
          defaultOpen={true}
          labels={{
            title: "Currency Intelligence Assistant",
            initial: "👋 Hi! I can help you convert currencies and look up exchange rates. Try asking for a conversion below!",
          } as any}
          suggestions={[
            {
              title: "Convert USD to EUR",
              message: "How much is 250 USD in EUR?",
            },
            {
              title: "Compare USD to GBP",
              message: "What is the exchange rate for USD to GBP?",
            },
            {
              title: "Convert CAD to USD",
              message: "Can you convert 1500 CAD to USD?",
            },
            {
              title: "Latest Rates",
              message: "Get the current exchange rate for USD to JPY.",
            },
          ]}
          messageView={CustomMessageView}
        />
        <YourMainContent themeColor={themeColor} setThemeColor={setThemeColor} />
      </main>
    </DebugContext.Provider>
  );
}

CustomMessageView.Cursor = CopilotChatMessageView.Cursor;

function YourMainContent({
  themeColor,
  setThemeColor,
}: {
  themeColor: string;
  setThemeColor: (color: string) => void;
}) {
  const { debug, setDebug } = React.useContext(DebugContext);
  const [activePayload, setActivePayload] = useState<unknown[] | null>(null);

  useEffect(() => {
    const handleSurfaceActive = (e: any) => {
      setActivePayload(e.detail.messages);
    };
    window.addEventListener("a2ui-surface-active", handleSurfaceActive);
    return () => window.removeEventListener("a2ui-surface-active", handleSurfaceActive);
  }, []);

  // 🪁 Generative UI: Renders the ExchangeRateCard dynamically during agent tool calls
  useRenderTool(
    {
      name: "get_exchange_rate",
      render: ({ parameters, result }: any) => {
        const from = (parameters as any)?.currency_from || "USD";
        const to = (parameters as any)?.currency_to || "EUR";
        const rate = (result as any)?.rates?.[to] || (result as any)?.rates?.[to.toUpperCase()];
        const date = (result as any)?.date;

        return (
          <ExchangeRateCard
            currencyFrom={from}
            currencyTo={to}
            rate={rate}
            date={date}
            themeColor={themeColor}
          />
        );
      },
    } as any,

    [themeColor],
  );

  return (
    <div
      style={{ backgroundColor: "#0f172a" }} // Matches deep dark-theme
      className="h-screen flex justify-center items-center flex-col transition-colors duration-300 px-6 text-center overflow-hidden"
    >
      {activePayload ? (
        <div className="w-full max-w-5xl h-[88vh] flex flex-col bg-slate-900/60 border border-slate-800/80 rounded-3xl p-8 shadow-2xl backdrop-blur-xl text-left animate-fade-in">
          {/* Dashboard Header */}
          <div className="flex justify-between items-center border-b border-slate-800/80 pb-4 mb-6">
            <div className="flex items-center gap-3">
              <span className="text-3xl filter drop-shadow-[0_2px_10px_rgba(16,185,129,0.3)]">📊</span>
              <div>
                <h1 className="text-2xl font-black text-transparent bg-clip-text bg-gradient-to-r from-emerald-400 via-teal-300 to-blue-400 leading-tight">
                  Currency Intelligence Workspace
                </h1>
                <p className="text-xs text-slate-400 mt-0.5">A2UI 1.0 viewport (Express + server-side macros, compiled in Rust)</p>
              </div>
            </div>
            <div className="flex items-center gap-4">
              {/* Palette */}
              <div className="flex gap-2 bg-slate-950/60 p-2 rounded-xl border border-slate-800/50">
                <button onClick={() => setThemeColor("#10b981")} className="w-4 h-4 rounded-full bg-emerald-500 hover:scale-125 transition duration-150 cursor-pointer" />
                <button onClick={() => setThemeColor("#3b82f6")} className="w-4 h-4 rounded-full bg-blue-500 hover:scale-125 transition duration-150 cursor-pointer" />
                <button onClick={() => setThemeColor("#ec4899")} className="w-4 h-4 rounded-full bg-pink-500 hover:scale-125 transition duration-150 cursor-pointer" />
              </div>
              <button
                onClick={() => setActivePayload(null)}
                className="px-4 py-2 rounded-xl text-xs font-bold text-slate-300 hover:text-white bg-slate-800/80 hover:bg-slate-700 border border-slate-700/60 transition cursor-pointer"
              >
                Reset Canvas
              </button>
            </div>
          </div>

          {/* Interactive viewport container */}
          <div className="flex-1 overflow-y-auto pr-2 custom-scrollbar">
            <A2uiMessages messages={activePayload} />
          </div>
        </div>
      ) : (
        <div className="max-w-md bg-slate-800/80 border border-slate-700 p-8 rounded-2xl shadow-2xl backdrop-blur-md">
          <span className="text-5xl mb-4 block">💵</span>
          <h1 className="text-3xl font-extrabold text-white tracking-tight mb-2">
            Currency Intellect
          </h1>
          <p className="text-slate-400 text-sm mb-6">
            Generative UI Workspace powered by CopilotKit and Rust Currency Agent. Ask the assistant to query rates.
          </p>
          <div className="flex justify-center gap-2">
            <button 
              onClick={() => setThemeColor("#10b981")} 
              className="w-8 h-8 rounded-full border-2 border-white/20 bg-emerald-500 cursor-pointer" 
            />
            <button 
              onClick={() => setThemeColor("#3b82f6")} 
              className="w-8 h-8 rounded-full border-2 border-white/20 bg-blue-500 cursor-pointer" 
            />
            <button 
              onClick={() => setThemeColor("#ec4899")} 
              className="w-8 h-8 rounded-full border-2 border-white/20 bg-pink-500 cursor-pointer" 
            />
          </div>

          {/* 🛠️ Debug Mode Switch */}
          <div className="flex flex-col items-center gap-2 mt-6 pt-4 border-t border-slate-700/50">
            <label className="flex items-center gap-3 cursor-pointer select-none">
              <span className="text-xs font-semibold tracking-wider uppercase text-slate-400">
                A2UI Debug Mode
              </span>
              <div className="relative">
                <input
                  type="checkbox"
                  checked={debug}
                  onChange={(e) => setDebug(e.target.checked)}
                  className="sr-only peer"
                />
                <div className="w-9 h-5 bg-slate-700 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full rtl:peer-checked:after:-translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:start-[2px] after:bg-white after:border-slate-300 after:border after:rounded-full after:h-4 after:w-4 after:transition-all peer-checked:bg-emerald-500" />
              </div>
            </label>
          </div>
        </div>
      )}
    </div>
  );
}

