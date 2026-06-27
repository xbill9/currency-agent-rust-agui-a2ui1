import { Hono } from "hono";
import { handle } from "hono/vercel";
import {
  CopilotRuntime,
  createCopilotRuntimeHandler,
  InMemoryAgentRunner,
} from "@copilotkit/runtime/v2";
import { HttpAgent } from "@ag-ui/client";

const runtime = new CopilotRuntime({
  agents: {
    currency_agent: new HttpAgent({
      url: process.env.AGENT_URL || "http://localhost:11000/apps/currency_agent",
    }),
  },
  runner: new InMemoryAgentRunner(),
  a2ui: {
    a2uiToolNames: ["render_a2ui", "send_a2ui_json_to_client"],
  },
});

const multiRouteHandler = createCopilotRuntimeHandler({
  runtime,
  basePath: "/api/copilotkit",
  mode: "multi-route",
  cors: true,
});

const singleRouteHandler = createCopilotRuntimeHandler({
  runtime,
  basePath: "/api/copilotkit",
  mode: "single-route",
  cors: true,
});

const app = new Hono().basePath("/api/copilotkit");

// Root path handler - resolves single-route requests (like POST /api/copilotkit with {method: "info"})
app.all("/", async (c) => singleRouteHandler(c.req.raw));

// Subpath handler - resolves multi-route requests (like GET /info and POST /agent/:agentId/run)
app.all("*", async (c) => {
  const path = c.req.path;
  if (path === "/api/copilotkit" || path === "/api/copilotkit/") {
    return singleRouteHandler(c.req.raw);
  }
  return multiRouteHandler(c.req.raw);
});

export const GET = handle(app);
export const POST = handle(app);
