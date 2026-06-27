---
title: "Extending a MCP/A2A Currency Agent with AG-UI and Antigravity CLI"
description: "Building an Agent with A2A, MCP, ADK, and AG-UI   This tutorial aims to extend and test a..."
published_at: "2026-06-12T16:50:26Z"
tags: ['googleadk', 'mcps', 'antigravitycli', 'agui']
canonical_url: "https://xbill999.medium.com/extending-a-mcp-a2a-currency-agent-with-ag-ui-and-antigravity-cli-9f3ef7c78c8c"
source_url: "https://dev.to/gde/extending-a-mcpa2a-currency-agent-with-ag-ui-and-antigravity-cli-4kd1"
---

---
title: Extending a MCP/A2A Currency Agent with AG-UI and Antigravity CLI
published: true
series: A2A
date: 2026-06-12 16:50:26 UTC
tags: googleadk,mcps,antigravitycli,agui
canonical_url: https://xbill999.medium.com/extending-a-mcp-a2a-currency-agent-with-ag-ui-and-antigravity-cli-9f3ef7c78c8c
---

#### Building an Agent with A2A, MCP, ADK, and AG-UI

This tutorial aims to extend and test a currency Agent using A2A and MCP protocols with the AG-UI protocol for custom presentation.

![](https://cdn-images-1.medium.com/max/1024/1*Ry3DC0DXHYXvco0i_iIuPA.jpeg)

#### Reduce, Re-Use, Re-Cycle!

This paper is a re-visiting of the original currency Agent Codelab:

[Getting Started with MCP, ADK and A2A | Google Codelabs](https://codelabs.developers.google.com/codelabs/currency-agent#0)

and a GitHub Repo:

[GitHub - jackwotherspoon/currency-agent: A sample agent demonstrating A2A + ADK + MCP working together.](https://github.com/jackwotherspoon/currency-agent)

In this updated version, the Antigravity CLI is used to add support for AG-UI and extend the existing user interface.

#### What is the A2A protocol?

The Agent2Agent (A2A) protocol, an open communication standard for AI agents, was initially introduced by Google in April 2025. It is specifically engineered to facilitate seamless interoperability within multi-agent systems, enabling AI agents developed by diverse providers or built upon disparate AI agent frameworks to communicate and collaborate effectively.

A good overview of the A2A protocol can be found here:

[A2A Protocol](https://a2a-protocol.org/latest/)

#### Language Support For the A2A Protocol

The official ADK for Python, GO, and Java provide built-in support for working with the A2A protocol. For other programming languages like JS, Rust, and .NET — 3rd party libraries are available to add support for the protocol.

The main source for A2A Language support is the GitHub repo:

[GitHub - a2aproject/A2A: An open protocol enabling communication and interoperability between opaque agentic applications.](https://github.com/a2aproject/A2A)

#### AG-UI

AG-UI (Agent-User Interaction) is an open-source protocol that standardizes how AI agents communicate with frontend web applications. It acts as a universal adapter between backend AI logic (like LangGraph or CrewAI) and user interfaces, handling streaming responses, tool executions, and state synchronization out of the box. [[1](https://github.com/ag-ui-protocol/ag-ui), [2](https://medium.com/@jatingargiitk/ag-ui-the-interface-protocol-for-human-agent-collaboration-a93025ab327c), [3](https://www.copilotkit.ai/blog/introducing-ag-ui-the-protocol-where-agents-meet-users), [4](https://techcommunity.microsoft.com/blog/appsonazureblog/ag-ui-the-future-of-agent-driven-user-interfaces/4515769)]

More details are available here:

[AG-UI Overview - Agent User Interaction Protocol](https://docs.ag-ui.com/introduction)

#### A2UI

A2UI (Agent-to-User Interface) is an open-source protocol that allows AI agents to dynamically generate and stream rich, interactive user interfaces in real-time. [[1](https://a2ui.org/introduction/what-is-a2ui/), [2](https://www.copilotkit.ai/blog/build-with-googles-new-a2ui-spec-agent-user-interfaces-with-a2ui-ag-ui)]

Instead of an AI relying on pre-built screens or just returning plain text in a chat window, A2UI enables the agent to instantly build and display tailored components like interactive charts, date-pickers, or approval forms. [[1](https://a2ui.org/introduction/what-is-a2ui/), [2](https://www.copilotkit.ai/blog/build-with-googles-new-a2ui-spec-agent-user-interfaces-with-a2ui-ag-ui)]

[What is A2UI? - A2UI](https://a2ui.org/introduction/what-is-a2ui/)

#### More Word Salad Protocols — What about A2A-XYZ?

This article provides a good overview of how the various protocols fit together:

[A2A, MCP, AG-UI, A2UI: The Essential 2026 AI Agent Protocol Stack](https://medium.com/@visrow/a2a-mcp-ag-ui-a2ui-the-essential-2026-ai-agent-protocol-stack-ee0e65a672ef)

Confused yet? But wait- there’s more!

[How Does A2UI Compare? - A2UI](https://a2ui.org/introduction/agent-ui-ecosystem/)

and this:

[AG-UI and A2UI: Understanding the Differences | CopilotKit](https://www.copilotkit.ai/ag-ui-and-a2ui)

#### Antigravity CLI

Antigravity CLI is the follow-on successor to Gemini CLI- the terminal driven, agent assisted coding tool.

Full details on installing Antigravity CLI are here:

[Getting Started with Antigravity CLI](https://medium.com/google-cloud/getting-started-with-antigravity-cli-26c5da90951f)

#### Testing the Antigravity CLI Environment

Once you have all the tools in place- you can test the startup of Antigravity CLI.

You will need to authenticate with a Google Cloud Project or your Google Account:

```plaintext
agy
```

This will start the interface:

![](https://cdn-images-1.medium.com/max/1024/1*lqIZz4wq1OG2KfDHUiEhcQ.png)

#### Checking the Developer Environment

Verify that all the prerequisite packages and compilers are installed — and clone the sample Github repo:

```shell
git clone https://github.com/xbill9/currency-agent-agui
cd currency-agent-agui
```

Once you have your Google Cloud Project and preferred authentication method — run the **init.sh** script to validate the setup:

```console
xbill@penguin:~/currency-agent-agui$ source init.sh

[environment: Development]
Updated property [core/project].
ADC is valid.
Environment setup
GOOGLE_GENAI_USE_VERTEXAI=1
GOOGLE_CLOUD_PROJECT=comglitn
GOOGLE_CLOUD_LOCATION=us-central1
IMAGEN_MODEL="imagen-3.0-fast-generate-001"
GENAI_MODEL="gemini-2.5-flash"
Cloud Login
  Credentialed Accounts
ACTIVE ACCOUNT
* xbill@glitnir.com
```

The **set\_env.sh** script is provided to set common ADK environment variables:

```console
xbill@penguin:~/currency-agent-agui$ source set_env.sh 
Current Environment
GOOGLE_GENAI_USE_VERTEXAI=false
GOOGLE_CLOUD_PROJECT=comglitn
GOOGLE_CLOUD_LOCATION=us-central1
GENAI_MODEL="gemini-2.5-flash"

Cloud Login
  Credentialed Accounts
ACTIVE ACCOUNT
* xbill@glitnir.com
```

#### Debugging API Permission Errors

If your application default credentials expires or your Google Cloud Authentication expires you will get an error. The workaround is to re-authenticate:

```shell
gcloud auth login
gcloud auth application-default login
```

Another common error is that the environment variables are not set correctly. Go the the root directory and re-run the **set\_env.sh** to set the variables:

```shell
cd ~/currency-agent-agui
source set_env.sh
```

#### A2A Debugging Tools -A2A Inspector

The A2A Inspector is a standalone tool that provides low level visibility into the A2A protocol. The GitHub is available here:

[GitHub - a2aproject/a2a-inspector: Validation Tools for A2A Agents](https://github.com/a2aproject/a2a-inspector)

A summary of the features of the A2A inspector can be found here:

[A2A Protocol Documentation](https://a2aprotocol.ai/docs/guide/a2a-inspector)

To install the A2A Inspector:

```shell
cd ~
git clone https://github.com/a2aproject/a2a-inspector
```

Then follow the build instructions — you need **_uv_** , and a recent version of  **_node_** :

[a2a-inspector/README.md at main · a2aproject/a2a-inspector](https://github.com/a2aproject/a2a-inspector/blob/main/README.md)

#### Staring the A2A Inspector

Once the A2A inspector has been installed- you can validate the installation by using this URL:

[http://127.0.0.1:5001/](http://127.0.0.1:5001/)

![](https://cdn-images-1.medium.com/max/1024/1*FEGNWHDblSb2Q14JGs5NFg.png)

#### Local A2A Validation with A2A Inspector

The local A2A agent is checked with the A2A Inspector:

```plaintext
> which port is the a2a server listening on

The A2A server is listening on port 10000 .

This is configured when starting the service (e.g., via uvicorn currency_agent.agent:a2a_app --port 10000 ).
```

and then the Agent card is retrieved:

![](https://cdn-images-1.medium.com/max/1024/1*ycM48PRFhN69_u0Nhu9F3A.png)

and the Agent skill cross checked:

![](https://cdn-images-1.medium.com/max/1024/1*OLgQxH_PkYuK9ctmRXmgzw.png)

#### Antigravity CLI with MCP Connection

The Master Agent starts a FastMCP server interface. This allows Antigravity CLI to be used as a MCP client:

```json
xbill@penguin:~/currency-agent-agui/.agents$ more mcp_config.json

"currency_mcp": {
      "serverUrl": "http://localhost:8080/mcp"
    }
```

The MCP tools can be reviewed:

```plaintext
MCP Servers

Plugins (~/.gemini/antigravity-cli/plugins)
   ✓ currency_mcp Tools: get_exchange_rate
```

The MCP tool functionality can be checked with Antigravity:

```markdown
> what does get_exchange_rate do?

I will search the repository to locate the definition of the get_exchange_rate tool.

● Search(Grep search) (ctrl+o to expand)

I will view the definition of the get_exchange_rate tool in server.py.

● Read(/home/xbill/currency-agent/mcp-server/server.py) (ctrl+o to expand)

The server.py tool is an MCP tool defined in server.py.

### What it does:

It retrieves the currency exchange rate between two currencies for a specific date (or the latest date) using the
  free Frankfurter API https://www.frankfurter.dev/.

### Parameters:

• currency_from ( str , default "USD" ): The source currency code (e.g. "USD" , "GBP" , "CAD" ).
  • currency_to ( str , default "EUR" ): The destination currency code to convert to (e.g. "EUR" , "JPY" , "AUD" 
  ).
  • currency_date ( str , default "latest" ): The date for the exchange rate query (formatted as YYYY-MM-DD or    
  "latest" ).

### Returns:

It returns a Python dictionary containing the Frankfurter API response:

{
      "amount": 1.0,
      "base": "USD",       
      "date": "2026-06-03",
      "rates": {      
        "EUR": 0.86103
      } 
    }

If an HTTP request fails or returns an invalid response, it catches the error and returns a dictionary detailing the
  error (e.g., {"error": "API request failed: ..."} ).
```

#### Where Does AG-UI come in?

The first version used a Vanilla TS interface to the backend agent system. AG-UI provides a standard way to look at the components:

![](https://cdn-images-1.medium.com/max/750/1*bXzCPtDBVBXYe69jgHJkcg.avif)

#### So What is all this Doing?

All of the setup and configuration allows you to start debugging and visualizing Agent flows using the A2A protocol. The currency Agent has been validated using the A2A Inspector and the MCP interface via Antigravity CLI.

The make help provides the main project targets:

```make
xbill@penguin:~/currency-agent-agui$ make help
Available commands:
  install - Install all project dependencies (including frontend and react UI)
  start - Start all services in background (MCP + Agent + AG-UI)
  stop - Stop all background services (including AG-UI)
  status - Check status of background services
  mcp - Start the MCP Server (foreground)
  agent - Start the A2A Agent Server (foreground)
  frontend - Build and start the FastAPI + Vanilla TS frontend server (port 8000)
  frontend-install - Install dependencies for FastAPI + Vanilla TS frontend
  frontend-build - Build Vanilla TS frontend assets
  react-install - Install dependencies for React + CopilotKit UI
  react-ui - Start React Frontend UI (port 3000)
  react-agent - Start React Frontend Agent (port 8008)
  test-client - Run the A2A Client (test queries)
  e2e-test - Run end-to-end tests (alias for test-client)
  adktest - Run interactive ADK CLI for the agent
  test - Run all tests (pytest)
  frontend-test - Run frontend specific tests
  react-test - Run end-to-end tests on React UI and React Agent
  test-agui - Run AG-UI CopilotKit Validator (validate_agui.py)
  lint - Run linting checks (ruff)
  format - Auto-format code (ruff)
  clean - Remove caches and logs
  deploy - Deploy to Cloud Run using Cloud Build
  logs - Read logs from Cloud Run
  endpoint - Get the Cloud Run service endpoint
  remote-status - Check the status of the remote endpoint
```

The Makefile provides targets to build and manage the project:

```shell
xbill@penguin:~/currency-agent-agui$ make install
Installing dependencies...
uv sync
Resolved 109 packages in 1ms
Checked 105 packages in 0.80ms
make frontend-install
make[1]: Entering directory '/home/xbill/currency-agent-agui'
Installing frontend dependencies...
cd frontend/frontend && npm install

up to date, audited 16 packages in 665ms

5 packages are looking for funding
  run `npm fund` for details

1 moderate severity vulnerability

To address all issues, run:
  npm audit fix

Run `npm audit` for details.
uv pip install -r frontend/requirements.txt
Checked 14 packages in 28ms
make[1]: Leaving directory '/home/xbill/currency-agent-agui'
make react-install
make[1]: Entering directory '/home/xbill/currency-agent-agui'
Installing React frontend dependencies...
cd frontend-react && npm install

> adk-starter@0.1.0 postinstall
> npm run install:agent


> adk-starter@0.1.0 install:agent
> ./scripts/setup-agent.sh || scripts\setup-agent.bat

Resolved 79 packages in 0.91ms
Checked 75 packages in 0.57ms

up to date, audited 1195 packages in 3s

232 packages are looking for funding
  run `npm fund` for details

9 vulnerabilities (5 low, 3 moderate, 1 high)

To address issues that do not require attention, run:
  npm audit fix

To address all issues (including breaking changes), run:
  npm audit fix --force

Run `npm audit` for details.
make[1]: Leaving directory '/home/xbill/currency-agent-agui'
```

Lint:

```shell
xbill@penguin:~/currency-agent-agui$ make lint
Running linting checks (ruff check + format)...
uv run ruff check .
All checks passed!
uv run ruff format --check .
21 files already formatted
```

Test:

```shell
xbill@penguin:~/currency-agent-agui$ make test
Running tests...
make stop
make[1]: Entering directory '/home/xbill/currency-agent-agui'
Stopping servers...
make[1]: *** [Makefile:68: stop] Killed
make[1]: Leaving directory '/home/xbill/currency-agent-agui'
make: [Makefile:194: test] Error 2 (ignored)
make start
make[1]: Entering directory '/home/xbill/currency-agent-agui'
Starting MCP Server in background...
Waiting for MCP Server to initialize...
Starting A2A Agent Server in background...
Starting AG-UI React Frontend UI in background...
Starting AG-UI React Frontend Agent in background...
Services started. Logs: mcp.log, agent.log, react-ui.log, react-agent.log
make[1]: Leaving directory '/home/xbill/currency-agent-agui'
uv run pytest
================================================ test session starts =================================================
platform linux -- Python 3.13.13, pytest-9.0.3, pluggy-1.6.0
rootdir: /home/xbill/currency-agent-agui
configfile: pyproject.toml
plugins: asyncio-1.3.0, anyio-4.10.0
asyncio: mode=Mode.STRICT, debug=False, asyncio_default_fixture_loop_scope=None, asyncio_default_test_loop_scope=function
collected 17 items
```

#### Time to Start some Currency Arbitrage!

The servers are started:

```shell
xbill@penguin:~/currency-agent-agui$ make start
Starting MCP Server in background...
Waiting for MCP Server to initialize...
Starting A2A Agent Server in background...
Starting AG-UI React Frontend UI in background...
Starting AG-UI React Frontend Agent in background...
Services started. Logs: mcp.log, agent.log, react-ui.log, react-agent.log
xbill@penguin:~/currency-agent-agui$ 

xbill@penguin:~/currency-agent-agui$ make status
Checking status of background services...
  MCP Server: Running (PID 5398 5505 )
  A2A Agent Server: Running (PID 5508 5665 )
  Frontend Server: Stopped
  AG-UI Agent: Running (PID 5769 )
  AG-UI React UI: Running (PID 5608 )
```

and checked end-to-end:

```shell
xbill@penguin:~/currency-agent-agui$ make react-test
Running React UI & Agent E2E tests...
make stop
make[1]: Entering directory '/home/xbill/currency-agent-agui'
Stopping servers...
make[1]: *** [Makefile:68: stop] Killed
make[1]: Leaving directory '/home/xbill/currency-agent-agui'
make: [Makefile:226: react-test] Error 2 (ignored)
make start
make[1]: Entering directory '/home/xbill/currency-agent-agui'
Starting MCP Server in background...
Waiting for MCP Server to initialize...
Starting A2A Agent Server in background...
Starting AG-UI React Frontend UI in background...
Starting AG-UI React Frontend Agent in background...
Services started. Logs: mcp.log, agent.log, react-ui.log, react-agent.log
make[1]: Leaving directory '/home/xbill/currency-agent-agui'
uv run python3 frontend-react/scripts/test_react_e2e.py

==================================================
🚀 Running React UI & Agent End-to-End Test
==================================================

=== Test Case 1: Checking Servers Health ====
✓ React Agent on port 8008 is healthy.
✓ React UI Server on port 3000 is healthy and reachable.

=== Test Case 2: Streaming Query to Currency Agent via Next.js ====
i Sending POST request to Next.js API route: [http://localhost:3000/api/copilotkit/agent/currency\_agent/run](http://localhost:3000/api/copilotkit/agent/currency_agent/run)
✓ Connected to SSE stream. Parsing events...
i Event: RUN_STARTED
i Event: TOOL_CALL_START -> get_exchange_rate
✓ SSE Stream completed successfully.
✓ The agent successfully invoked a tool during execution.
i Agent response:
<a2ui-json>
{
  "version": "v0.9",
  "createSurface": {
    "surfaceId": "currency_conversion_surface",
    "catalogId": "https://a2ui.org/specification/v0_9/basic_catalog.json",
    "theme": {
      "agentDisplayName": "Currency Agent"
    }
  }
}
</a2ui-json>
<a2ui-json>
{
  "version": "v0.9",
  "updateComponents": {
    "surfaceId": "currency_conversion_surface",
    "components": [
      {
        "id": "root",
        "component": "Card",
        "child": "conversion_text"
      },
      {
        "id": "conversion_text",
        "component": "Text",
        "text": "100 USD is equal to 85.91 EUR."
      }
    ]
  }
}
</a2ui-json>
✓ Agent streamed content back to UI successfully.

==================================================
🎉 ALL REACT E2E TEST CHECKS PASSED SUCCESSFULLY!

xbill@penguin:~/currency-agent-agui$ make start
Starting MCP Server in background...
Waiting for MCP Server to initialize...
Starting A2A Agent Server in background...
Starting AG-UI React Frontend UI in background...
Starting AG-UI React Frontend Agent in background...
Services started. Logs: mcp.log, agent.log, react-ui.log, react-agent.log
xbill@penguin:~/currency-agent-agui$ make test-agui
Running AG-UI CopilotKit Validator...
uv run python3 frontend-react/scripts/validate_agui.py
i Both port 3000 (Next.js) and 8008 (React Agent) are open. Defaulting validation to 'nextjs'.

==================================================
🚀 Running AG-UI CopilotKit Validator on target: NEXTJS
==================================================

=== Test Case 1: Health & Reachability ====
✓ Next.js server is reachable at [http://localhost:3000/api/copilotkit](http://localhost:3000/api/copilotkit) (HTTP 405)

=== Test Case 2: Capabilities Discovery ====
i Skipping capabilities GET check for Next.js (queried internally by CopilotKit runtime)

=== Test Case 3: Streamed Currency Conversion & A2UI Output ====
i Sending conversion request and reading SSE stream...
i Tool execution started: get_exchange_rate
✓ SSE Stream completed with valid flow transitions.
✓ A2UI JSON payload extracted and successfully parsed as valid JSON.
  Extracted Components: [{"id": "root", "component": "Card", "child": "conversion_details"}, {"id": "conversion_details", "component": "Column", "children": ["conversion_titl...

=== Test Case 4: Agent Guardrails & Domain Enforcement ====
i Sending off-topic request and reading SSE stream...
✓ Agent correctly refused to answer off-topic prompt (Matched: ['only assist', 'currency'])
  Response text: I can only assist with currency-related queries.

=== Test Case 5: Thread State Retrieval ====
i Skipping thread state retrieval check for Next.js (not exposed on API route)

==================================================
🎉 ALL INTEGRATION VALIDATION CHECKS PASSED SUCCESSFULLY!
```

#### Start the Currency Session

Once all the agents have been started — the react interface on port 3000 is connected with a standard web browser:

![](https://cdn-images-1.medium.com/max/1024/1*M4vda8w6Pr71Ssp1rMQ6Vg.png)

The new AG-UI tools are presented:

![](https://cdn-images-1.medium.com/max/482/1*jnSy4uBmpa0tyWB2el6uEQ.png)

And in plain text:

```plaintext
Exchange Rate Details
Date: 6/5/2026

💵
From
USD
➔
To
JPY
Fetching...
1 USD = ? JPY
1 USD is equal to 159.86 JPY (as of 2026-06-05).
```

The Co-Pilot chat-style box allows for more complex queries:

![](https://cdn-images-1.medium.com/max/477/1*kcW7mb069ywxVGgKhMv35w.png)

and the results are presented:

```plaintext
USD Exchange Rates (as of 2026-06-05)
Currency
Rate (1 USD = X)
CAD

1.3882

THB

32.635

GBP

0.74255

JPY

159.86
```

#### So how does AG-UI actually work?

Antigravity was used for a detailed breakdown:

```typescript
> give some examples of how the ag-ui code works in the current currency project focus on the react interface

● Read(/home/xbill/currency-agent-agui/frontend-react/src/components/CopilotProvider.tsx) (ctrl+o to expand)

▸ Thought Process
  Analyzing A2UI Provider
  In the React frontend ( frontend-react/ ), the AG-UI / A2UI framework is structured around three main parts:
  catalog registration, message streaming interception, and surface rendering.
  Here is how each layer works in the React interface:
  ──────
  ### 1. Catalog Registration ( A2UIProvider )

  In CopilotProvider.tsx, the React application is wrapped in <A2UIProvider> :

    import { A2UIProvider } from "@copilotkit/a2ui-renderer";                                                      
    import { customCatalog } from "./A2UICustomCatalog";                                                           
                                                                                                                   
    export default function CopilotProvider({ children }) {                                                        
      return (                                                                                                     
        <CopilotKit runtimeUrl="/api/copilotkit" agent="currency_agent">                                           
          <A2UIProvider catalog={customCatalog}>                                                                   
            {children}                                                                                             
          </A2UIProvider>                                                                                          
        </CopilotKit>                                                                                              
      );                                                                                                           
    }                                                                                                              

  • customCatalog : Defined in A2UICustomCatalog.tsx, this is built using createCatalog() . It specifies the
component
  props (using Zod schemas) and associates them with custom React elements (like Card , Table , BarChart , and  
  LineChart ).
  ──────
  ### 2. Message Streaming Interception

  In page.tsx, a custom markdown renderer intercepts the assistant's stream:

    const CustomMarkdownRenderer = ({ content, ...props }: any) => {                                               
      const { cleanText, parsed, surfaceId } = parseA2UIContent(content);                                          
      return (                                                                                                     
        <div className="flex flex-col gap-2 w-full">                                                               
          <CopilotChatAssistantMessage.MarkdownRenderer                                                            
            {...props}                                                                                             
            content={cleanText}                                                                                    
          />                                                                                                       
          {parsed && surfaceId && (                                                                                
            <A2UIContainer parsedJson={parsed} surfaceId={surfaceId} />                                            
          )}                                                                                                       
        </div>                                                                                                     
      );                                                                                                           
    };                                                                                                             

  • parseA2UIContent : Strips the <a2ui-json> block from the text content (so JSON code never shows up in the
  chatbot bubbles) and parses it into a valid JavaScript object.
  • cleanText : Rendered as standard markdown message bubbles.
  • A2UIContainer : Mounted only when valid A2UI JSON payload exists in the response.
  ──────
  ### 3. Surface Rendering & State Synchronization

  In page.tsx, the container handles component injection and rendering:

    const A2UIContainer = ({ parsedJson, surfaceId }: { parsedJson: any; surfaceId: string }) => {                 
      const { processMessages, getSurface } = useA2UI();                                                           
                                                                                                                   
      useEffect(() => {                                                                                            
        if (parsedJson) {                                                                                          
          const messagesToProcess = [];                                                                            
                                                                                                                   
          // 1. If the surface isn't created yet, synthesize a createSurface message                               
          if (parsedJson.updateComponents && surfaceId && !getSurface(surfaceId)) {                                
            messagesToProcess.push({                                                                               
              version: "0.9",                                                                                      
              createSurface: {                                                                                     
                surfaceId: surfaceId,                                                                              
                catalogId: "https://a2ui.org/specification/v0_9/basic_catalog.json"
              }
            });
          }

          // 2. Queue the updateComponents instructions
          messagesToProcess.push(parsedJson);
          processMessages(messagesToProcess);
        }
      }, [parsedJson, surfaceId, processMessages, getSurface]);

      // 3. Render the synchronized surface UI natively
      return (
        <div className="mt-3 w-full">
          <A2UIRenderer surfaceId={surfaceId} />
        </div>
      );
    };

  • processMessages : Dynamically updates the A2UI state machine on the client side with components sent from the
  agent.
  • <A2UIRenderer surfaceId={surfaceId} /> : Renders the specific component tree for that surface by pulling
  matched components from the configured catalog.
```

#### Summary

The goal of the demo/article was to enable AG-UI integration on a functional agent supporting common protocols like A2A and MCP.

The currency agent was chosen as a starting point and new AG-UI presentation objects were added to the user interface with Antigravity CLI.

This interface was tested with a frontend local React server.

Finally- the updated project integration was summarized, and code reviewed with Antigravity CLI.