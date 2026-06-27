# AG-UI / A2UI Development & Code Samples

[AG-UI (Agent-User Interaction)](https://docs.ag-ui.com/introduction) and [A2UI (Agent-to-User Interface)](https://a2ui.org) allow backend AI agents to send structured, declarative UI schemas (like cards, tables, charts, or forms) instead of plain text. The client-side application then renders these components natively and synchronizes state bidirectionally.

Below are full, production-ready sample implementations for both the **Python ADK Backend Agent** and the **React + CopilotKit Frontend Client**, as well as component catalog definition and payload examples.

---

## 1. Custom Catalog Definition (`custom_catalog.json`)
Before the agent can generate UI, components must be defined with JSON Schema.

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "catalogId": "https://a2ui.org/catalog/currency-charts",
  "name": "currency-charts",
  "components": {
    "BarChart": {
      "description": "Renders a vertical bar chart for showing historical exchange rates.",
      "properties": {
        "title": { "type": "string" },
        "labels": {
          "type": "array",
          "items": { "type": "string" }
        },
        "values": {
          "type": "array",
          "items": { "type": "number" }
        },
        "color": { "type": "string", "default": "#10b981" }
      },
      "required": ["labels", "values"]
    }
  }
}
```

---

## 2. Python Backend Agent (`agent.py`)
This script uses the **Google ADK** and **A2UI Agent SDK** to construct the agent system prompt, automatically embedding the schemas and few-shot examples so the Gemini model learns how to generate valid A2UI JSON.

```python
import os
from dotenv import load_dotenv
from google.adk.agents import LlmAgent
from google.adk.tools.mcp_tool import McpToolset, StreamableHTTPConnectionParams
from google.adk.a2a.utils.agent_to_a2a import to_a2a
from google.adk.a2a.executor.a2a_agent_executor import A2aAgentExecutor, A2aAgentExecutorConfig

# A2UI SDK imports
from a2ui.schema.manager import A2uiSchemaManager
from a2ui.basic_catalog.provider import BasicCatalog
from a2ui.schema.catalog import CatalogConfig
from a2ui.adk.send_a2ui_to_client_toolset import SendA2uiToClientToolset, A2uiPartConverter

load_dotenv()

# 1. Load catalogs (BasicCatalog + Custom Catalog)
schema_manager = A2uiSchemaManager(
    version="0.9",
    catalogs=[
        BasicCatalog.get_config("0.9"),
        CatalogConfig.from_path(
            name="currency-charts",
            catalog_path=os.path.join(os.path.dirname(__file__), "custom_catalog.json"),
        ),
    ],
)
selected_catalog = schema_manager.get_selected_catalog()
examples_str = schema_manager.load_examples(selected_catalog)

# 2. Automatically generate the system instruction including UI schemas
SYSTEM_INSTRUCTION = schema_manager.generate_system_prompt(
    role_description="You are a specialized currency assistant that provides converted rates.",
    workflow_description="Use components to display structured currency conversions and rates.",
    ui_description="Use Card and Text components for displaying rates. Use BarChart for trends.",
    include_schema=True,
    include_examples=True,
    allowed_components=["Card", "Text", "Table", "BarChart"],
)

a2ui_toolset = SendA2uiToClientToolset(
    a2ui_enabled=True,
    a2ui_catalog=selected_catalog,
    a2ui_examples=examples_str,
)

# 3. Create the LLM Agent
root_agent = LlmAgent(
    model=os.getenv("GENAI_MODEL", "gemini-2.5-flash"),
    name="currency_agent",
    description="An agent that converts currencies and generates rich UI.",
    instruction=SYSTEM_INSTRUCTION,
    tools=[a2ui_toolset],
)

# 4. Wrap with Agent-to-Agent (A2A) compatibility
def agent_executor_factory(runner):
    config = A2aAgentExecutorConfig(
        gen_ai_part_converter=A2uiPartConverter(
            selected_catalog, bypass_tool_check=True
        ).convert
    )
    return A2aAgentExecutor(runner=runner, config=config)

a2a_app = to_a2a(
    root_agent,
    host="127.0.0.1",
    port=10000,
    agent_executor_factory=agent_executor_factory,
)
```

---

## 3. React Frontend Catalog (`A2UICustomCatalog.tsx`)
On the React client, you map component schemas to custom React/Tailwind elements using `@copilotkit/a2ui-renderer`.

```tsx
import React from "react";
import { z } from "zod";
import { createCatalog } from "@copilotkit/a2ui-renderer";

const customCatalogDefinitions = {
  Card: {
    description: "A card container",
    props: z.object({
      title: z.string().optional(),
      child: z.string().optional(),
      children: z.array(z.string()).optional(),
    }),
  },
  BarChart: {
    description: "A vertical bar chart for showing rates",
    props: z.object({
      title: z.string().optional(),
      labels: z.array(z.string()),
      values: z.array(z.number()),
      color: z.string().optional(),
    }),
  },
};

const customCatalogRenderers = {
  Card: ({ props, children }: any) => {
    return (
      <div className="bg-slate-800 border border-slate-700 rounded-xl p-4 my-2 shadow-xl text-white">
        {props.title && <h4 className="text-sm font-bold pb-2 mb-3 border-b border-slate-700">{props.title}</h4>}
        {props.child && children(props.child)}
        {props.children?.map((id: string) => (
          <React.Fragment key={id}>{children(id)}</React.Fragment>
        ))}
      </div>
    );
  },
  BarChart: ({ props }: any) => {
    const { title, labels, values, color = "#10b981" } = props;
    return (
      <div className="bg-slate-900 border border-slate-800 rounded-xl p-4 mt-2">
        {title && <h5 className="text-white text-sm font-semibold mb-3">{title}</h5>}
        <div className="flex items-end justify-around h-[120px] py-2 border-b border-slate-700">
          {labels.map((label: string, idx: number) => {
            const val = values[idx] ?? 0;
            return (
              <div key={idx} className="flex flex-col items-center flex-1">
                <span className="text-[10px] text-white font-medium">{val.toFixed(2)}</span>
                <div
                  style={{ height: `${(val / Math.max(...values)) * 100}%`, backgroundColor: color }}
                  className="w-5 rounded-t transition-all duration-300"
                />
                <span className="text-[10px] text-slate-400 mt-1">{label}</span>
              </div>
            );
          })}
        </div>
      </div>
    );
  },
};

export const customCatalog = createCatalog(customCatalogDefinitions as any, customCatalogRenderers as any, {
  catalogId: "https://a2ui.org/specification/v0_9/basic_catalog.json"
});
```

---

## 4. React Main App Integration (`page.tsx`)
To render the components, parse the `<a2ui-json>` markup in the chatbot message stream and feed it into `useA2UI`.

```tsx
"use client";

import React, { useEffect } from "react";
import { CopilotSidebar, CopilotChatMessageView, CopilotChatAssistantMessage } from "@copilotkit/react-core/v2";
import { useA2UI, A2UIRenderer } from "@copilotkit/a2ui-renderer";

function parseA2UIContent(text: string) {
  let cleanText = text;
  let parsed: any = null;
  const xmlStartIndex = text.indexOf("<a2ui-json>");
  
  if (xmlStartIndex !== -1) {
    cleanText = text.substring(0, xmlStartIndex);
    const xmlMatch = text.match(/<a2ui-json>([\s\S]*?)<\/a2ui-json>/);
    if (xmlMatch) {
      try {
        parsed = JSON.parse(xmlMatch[1].trim());
      } catch (e) {}
    }
  }
  return { cleanText, parsed };
}

const A2UIContainer = ({ parsedJson }: { parsedJson: any }) => {
  const { processMessages, getSurface } = useA2UI();
  const surfaceId = parsedJson.updateComponents?.surfaceId || "my_surface";

  useEffect(() => {
    if (parsedJson) {
      const messages = [];
      // If surface doesn't exist, create it first
      if (!getSurface(surfaceId)) {
        messages.push({
          version: "0.9",
          createSurface: {
            surfaceId: surfaceId,
            catalogId: "https://a2ui.org/specification/v0_9/basic_catalog.json"
          }
        });
      }
      messages.push(parsedJson);
      processMessages(messages);
    }
  }, [parsedJson, processMessages, getSurface]);

  return <A2UIRenderer surfaceId={surfaceId} />;
};

const CustomMarkdownRenderer = ({ content, ...props }: any) => {
  const { cleanText, parsed } = parseA2UIContent(content);
  return (
    <div className="w-full">
      <CopilotChatAssistantMessage.MarkdownRenderer {...props} content={cleanText} />
      {parsed && <A2UIContainer parsedJson={parsed} />}
    </div>
  );
};

export default function ChatPage() {
  return (
    <CopilotSidebar
      defaultOpen={true}
      messageView={(props: any) => (
        <CopilotChatMessageView
          {...props}
          assistantMessage={(msgProps: any) => (
            <CopilotChatAssistantMessage
              {...msgProps}
              markdownRenderer={CustomMarkdownRenderer}
            />
          )}
        />
      )}
    />
  );
}
```

---

## 5. Sample Agent Output Payload
When the backend agent returns a UI component, it appends a structured JSON block wrapped in `<a2ui-json>` to its text output:

```html
Here is the rate analysis for USD to EUR:

<a2ui-json>
{
  "version": "0.9",
  "updateComponents": {
    "surfaceId": "currency_agent",
    "components": [
      {
        "id": "card_usd_eur",
        "type": "Card",
        "props": {
          "title": "USD to EUR Conversion Rate Analysis",
          "children": ["text_info", "chart_rates"]
        }
      },
      {
        "id": "text_info",
        "type": "Text",
        "props": {
          "value": "Showing the historical trend of 1 USD in EUR over the last 3 days.",
          "variant": "body"
        }
      },
      {
        "id": "chart_rates",
        "type": "BarChart",
        "props": {
          "title": "Exchange Rate History",
          "labels": ["June 3", "June 4", "June 5"],
          "values": [0.915, 0.921, 0.918],
          "color": "#10b981"
        }
      }
    ]
  }
}
</a2ui-json>
```
