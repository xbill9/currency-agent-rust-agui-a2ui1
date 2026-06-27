"use client";

import { ExchangeRateCard } from "@/components/currency";
import {
  useFrontendTool,
  useRenderTool,
  CopilotSidebar,
  CopilotChatMessageView,
  CopilotChatAssistantMessage,
} from "@copilotkit/react-core/v2";
import { useA2UI, A2UIRenderer } from "@copilotkit/a2ui-renderer";
import React, { useState, useEffect } from "react";
import { z } from "zod";

function normalizeA2UIJson(parsed: any) {
  if (!parsed) return parsed;

  let components: any[] = [];
  if (Array.isArray(parsed)) {
    components = parsed;
  } else if (parsed && Array.isArray(parsed.components)) {
    components = parsed.components;
  } else if (parsed && parsed.updateComponents && Array.isArray(parsed.updateComponents.components)) {
    components = parsed.updateComponents.components;
  } else if (parsed && parsed.message && Array.isArray(parsed.message.components)) {
    components = parsed.message.components;
  } else {
    return parsed;
  }

  const normalized: any[] = [];
  components.forEach((c: any) => {
    const norm: any = { ...c };
    
    // Ensure component / type reciprocal matching
    if (!norm.type && norm.component) {
      norm.type = norm.component;
    }
    if (!norm.component && norm.type) {
      norm.component = norm.type;
    }
    if (!norm.props) {
      norm.props = {};
    }

    // Bidirectional sync for Text component properties (props.value/text <-> top-level text)
    if (norm.component === "Text" || norm.type === "Text") {
      const textValue = norm.text || norm.props.text || norm.props.value || "";
      norm.text = textValue;
      norm.props.text = textValue;
      norm.props.value = textValue;

      const variantValue = norm.variant || norm.props.variant || "body";
      norm.variant = variantValue;
      norm.props.variant = variantValue;
    }

    // Bidirectional sync for Card component properties (props.child/children <-> top-level child)
    if (norm.component === "Card" || norm.type === "Card") {
      const titleValue = norm.title || norm.props.title || "💵 Conversion Result";
      norm.title = titleValue;
      norm.props.title = titleValue;

      let childId = norm.child || norm.props.child;
      if (!childId && Array.isArray(norm.props.children) && norm.props.children.length > 0) {
        childId = norm.props.children[0];
      }
      if (!childId && Array.isArray(norm.children) && norm.children.length > 0) {
        childId = norm.children[0];
      }

      // Standard basic catalog requires Card to have a "child" top-level ID.
      // If we don't have one, synthesize a default or use the first child.
      if (!childId) {
        childId = norm.id + "_child";
      }

      norm.child = childId;
      norm.props.child = childId;

      if (!norm.props.children) {
        norm.props.children = [childId];
      }
    }

    // Robust bidirectional sync for all standard/custom properties (top-level <-> props)
    const keysToSync = [
      "variant",
      "title",
      "child",
      "children",
      "headers",
      "rows",
      "labels",
      "values",
      "color",
      "text",
      "value",
      "primary",
      "action",
      "series"
    ];
    keysToSync.forEach((key) => {
      if (norm[key] !== undefined && norm.props[key] === undefined) {
        norm.props[key] = norm[key];
      } else if (norm.props[key] !== undefined && norm[key] === undefined) {
        norm[key] = norm.props[key];
      }
    });

    normalized.push(norm);
  });

  // Resolve parent-child relationships using "child" or "children"
  normalized.forEach((c: any) => {
    if (c.child) {
      const childId = c.child;
      const childComp = normalized.find(x => x.id === childId);
      if (childComp && childComp.parentId === undefined) {
        childComp.parentId = c.id;
      }
    }
    if (c.props?.child) {
      const childId = c.props.child;
      const childComp = normalized.find(x => x.id === childId);
      if (childComp && childComp.parentId === undefined) {
        childComp.parentId = c.id;
      }
    }
    if (Array.isArray(c.children)) {
      c.children.forEach((childId: any) => {
        const childComp = normalized.find(x => x.id === childId);
        if (childComp && childComp.parentId === undefined) {
          childComp.parentId = c.id;
        }
      });
    }
    if (Array.isArray(c.props?.children)) {
      c.props.children.forEach((childId: any) => {
        const childComp = normalized.find(x => x.id === childId);
        if (childComp && childComp.parentId === undefined) {
          childComp.parentId = c.id;
        }
      });
    }
  });

  // Back-populate parent's props.children if a child has parentId
  normalized.forEach((c: any) => {
    if (c.parentId !== undefined) {
      const parent = normalized.find(x => x.id === c.parentId);
      if (parent) {
        if (!parent.props) {
          parent.props = {};
        }
        if (!parent.props.children) {
          parent.props.children = [];
        }
        if (Array.isArray(parent.props.children) && !parent.props.children.includes(c.id)) {
          parent.props.children.push(c.id);
        }
      }
    }
  });

  // Ensure we have a "root" component to satisfy A2UI/A2UIRenderer requirements
  const hasRoot = normalized.some(x => x.id === "root");
  if (!hasRoot && normalized.length > 0) {
    const topLevelComponents = normalized.filter(x => x.parentId === undefined);
    if (topLevelComponents.length === 1) {
      topLevelComponents[0].id = "root";
    } else if (topLevelComponents.length > 1) {
      // Create a wrapper Column with id "root"
      const rootWrapper = {
        id: "root",
        type: "Column",
        component: "Column",
        props: {
          children: topLevelComponents.map(x => x.id)
        }
      };
      // Mark topLevelComponents as having root as parent
      topLevelComponents.forEach(x => {
        x.parentId = "root";
      });
      normalized.unshift(rootWrapper);
    } else {
      normalized[0].id = "root";
    }
  }

  const result = {
    version: parsed.version || "v0.9",
  } as any;

  if (parsed.updateComponents) {
    result.updateComponents = {
      ...parsed.updateComponents,
      components: normalized,
    };
  } else if (parsed.createSurface) {
    result.createSurface = {
      ...parsed.createSurface,
    };
  } else {
    result.updateComponents = {
      surfaceId: parsed.surfaceId || "currency_agent",
      components: normalized,
    };
  }

  return result;
}

function cleanPartialJson(str: string): string {
  let inString = false;
  let isEscaped = false;
  const stack: string[] = [];

  for (let i = 0; i < str.length; i++) {
    const char = str[i];
    if (isEscaped) {
      isEscaped = false;
      continue;
    }
    if (char === '\\') {
      isEscaped = true;
      continue;
    }
    if (char === '"') {
      inString = !inString;
      continue;
    }
    if (!inString) {
      if (char === '{' || char === '[') {
        stack.push(char);
      } else if (char === '}') {
        if (stack[stack.length - 1] === '{') {
          stack.pop();
        }
      } else if (char === ']') {
        if (stack[stack.length - 1] === '[') {
          stack.pop();
        }
      }
    }
  }

  let closedStr = str;
  if (inString) {
    closedStr += '"';
  }

  while (stack.length > 0) {
    const lastOpen = stack.pop();
    if (lastOpen === '{') {
      closedStr = closedStr.trim();
      if (closedStr.endsWith(',') || closedStr.endsWith(':')) {
        closedStr = closedStr.slice(0, -1);
      }
      closedStr += '}';
    } else if (lastOpen === '[') {
      closedStr = closedStr.trim();
      if (closedStr.endsWith(',')) {
        closedStr = closedStr.slice(0, -1);
      }
      closedStr += ']';
    }
  }

  return closedStr;
}

function parseA2UIContent(text: string) {
  let cleanText = text;
  let jsonStr: string | null = null;
  let surfaceId = "currency_agent"; // default surface ID

  // 1. Cut off streaming XML/JSON from cleanText so it doesn't leak during streaming
  const xmlStartIndex = text.indexOf("<a2ui-json>");
  if (xmlStartIndex !== -1) {
    cleanText = text.substring(0, xmlStartIndex);
    const xmlEndIndex = text.indexOf("</a2ui-json>");
    if (xmlEndIndex !== -1) {
      jsonStr = text.substring(xmlStartIndex + 11, xmlEndIndex);
    } else {
      jsonStr = text.substring(xmlStartIndex + 11);
    }
  } else {
    // Check if there is a raw JSON block starting with { and containing "version": "v0.9" or "0.9"
    const jsonStartIndex = text.search(/\{[\s\S]*?"version"\s*:\s*"v?0.9"/);
    if (jsonStartIndex !== -1) {
      cleanText = text.substring(0, jsonStartIndex);
      const jsonMatch = text.match(/(\{[\s\S]*?"version"\s*:\s*"v?0.9"[\s\S]*?\})/);
      if (jsonMatch) {
        jsonStr = jsonMatch[1];
      } else {
        jsonStr = text.substring(jsonStartIndex);
      }
    }
  }

  let parsed: any = null;
  if (jsonStr) {
    try {
      const cleaned = cleanPartialJson(jsonStr.trim());
      const rawParsed = JSON.parse(cleaned);
      parsed = normalizeA2UIJson(rawParsed);
      if (parsed.updateComponents?.surfaceId) {
        surfaceId = parsed.updateComponents.surfaceId;
      } else if (parsed.createSurface?.surfaceId) {
        surfaceId = parsed.createSurface.surfaceId;
      }
    } catch (e) {
      // JSON is not yet complete/valid
    }
  }

  return { cleanText, parsed, surfaceId };
}


const A2UIContainer = ({ parsedJson, surfaceId }: { parsedJson: any; surfaceId: string }) => {
  const { processMessages, getSurface } = useA2UI();
  const lastProcessedRef = React.useRef<string>("");

  useEffect(() => {
    if (parsedJson) {
      const serialized = JSON.stringify(parsedJson);
      if (serialized === lastProcessedRef.current) {
        return;
      }
      lastProcessedRef.current = serialized;

      const messagesToProcess = [];
      // Synthesize createSurface if the surface does not exist yet in A2UI store
      if (parsedJson.updateComponents && surfaceId && !getSurface(surfaceId)) {
        messagesToProcess.push({
          version: "v0.9",
          createSurface: {
            surfaceId: surfaceId,
            catalogId: "https://a2ui.org/specification/v0_9/basic_catalog.json"
          }
        });
      }
      messagesToProcess.push(parsedJson);
      processMessages(messagesToProcess);

      // Dispatch event to render/update the main page dashboard workspace
      const updateEvent = new CustomEvent("a2ui-surface-active", { detail: { surfaceId, parsedJson } });
      window.dispatchEvent(updateEvent);
    }
  }, [parsedJson, surfaceId, processMessages, getSurface]);

  return (
    <div className="mt-3 w-full">
      <A2UIRenderer surfaceId={surfaceId} />
    </div>
  );
};


const DebugContext = React.createContext<{ debug: boolean; setDebug: (d: boolean) => void }>({
  debug: false,
  setDebug: () => {},
});

const CustomMarkdownRenderer = ({ content, ...props }: any) => {
  const { debug } = React.useContext(DebugContext);
  const { cleanText, parsed, surfaceId } = React.useMemo(
    () => parseA2UIContent(content),
    [content]
  );
  return (
    <div className="flex flex-col gap-2 w-full">
      <CopilotChatAssistantMessage.MarkdownRenderer
        {...props}
        content={cleanText}
      />
      {parsed && surfaceId && (
        <A2UIContainer parsedJson={parsed} surfaceId={surfaceId} />
      )}
      {debug && parsed && (
        <div className="mt-2 p-3 bg-slate-950/80 border border-slate-800 rounded-xl font-mono text-[10px] text-emerald-400 max-h-[250px] overflow-auto text-left select-all">
          <div className="text-[9px] uppercase tracking-wider text-slate-500 font-bold mb-1 border-b border-slate-900 pb-1">
            Raw A2UI JSON payload
          </div>
          <pre>{JSON.stringify(parsed, null, 2)}</pre>
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
  const [activeSurfaceId, setActiveSurfaceId] = useState<string | null>(null);
  const [activePayload, setActivePayload] = useState<any | null>(null);

  useEffect(() => {
    const handleSurfaceActive = (e: any) => {
      const { surfaceId, parsedJson } = e.detail;
      console.log("Widescreen dashboard received active surface:", surfaceId);
      setActiveSurfaceId(surfaceId);
      setActivePayload(parsedJson);
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
      {activeSurfaceId ? (
        <div className="w-full max-w-5xl h-[88vh] flex flex-col bg-slate-900/60 border border-slate-800/80 rounded-3xl p-8 shadow-2xl backdrop-blur-xl text-left animate-fade-in">
          {/* Dashboard Header */}
          <div className="flex justify-between items-center border-b border-slate-800/80 pb-4 mb-6">
            <div className="flex items-center gap-3">
              <span className="text-3xl filter drop-shadow-[0_2px_10px_rgba(16,185,129,0.3)]">📊</span>
              <div>
                <h1 className="text-2xl font-black text-transparent bg-clip-text bg-gradient-to-r from-emerald-400 via-teal-300 to-blue-400 leading-tight">
                  Currency Intelligence Workspace
                </h1>
                <p className="text-xs text-slate-400 mt-0.5">Premium Agent-to-User Interface (A2UI) Viewport</p>
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
                onClick={() => {
                  setActiveSurfaceId(null);
                  setActivePayload(null);
                }}
                className="px-4 py-2 rounded-xl text-xs font-bold text-slate-300 hover:text-white bg-slate-800/80 hover:bg-slate-700 border border-slate-700/60 transition cursor-pointer"
              >
                Reset Canvas
              </button>
            </div>
          </div>

          {/* Interactive viewport container */}
          <div className="flex-1 overflow-y-auto pr-2 custom-scrollbar">
            <A2UIRenderer surfaceId={activeSurfaceId} />
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

