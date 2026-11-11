"use client";

// Renders one batch of A2UI 1.0 agent-to-renderer messages (one compiled
// Express block from the Rust agent). Each batch gets its own
// MessageProcessor, so every chat turn's surface stays independent.

import React, { useEffect, useMemo, useState } from "react";
import { A2uiSurface, type ReactCatalogComponent } from "@a2ui/react";
import { MessageProcessor, type ActionPayload, type SurfaceModel } from "@a2ui/web_core/v1_0";
import { a2uiCatalogs, ensureA2uiSetup } from "./A2UICustomCatalog";

/** Hands a v1.0 action to the page, which turns it into a chat message. */
function dispatchAction(action: ActionPayload) {
  window.dispatchEvent(
    new CustomEvent("a2ui-action", { detail: { name: action.name, params: action.context } }),
  );
}

export function A2uiMessages({ messages }: { messages: unknown[] }) {
  ensureA2uiSetup();
  // A tool result or message can arrive as a new array on every render (and
  // React runs effects twice in dev), so key the processor on the payload and
  // apply it exactly once: re-sending createSurface for a live surface fails.
  const key = JSON.stringify(messages);
  const { processor, error } = useMemo(() => {
    const p = new MessageProcessor<ReactCatalogComponent>(a2uiCatalogs as any, dispatchAction);
    try {
      p.processMessages(JSON.parse(key));
      return { processor: p, error: null as string | null };
    } catch (e) {
      return { processor: p, error: e instanceof Error ? e.message : String(e) };
    }
  }, [key]);
  const [surfaces, setSurfaces] = useState<SurfaceModel<ReactCatalogComponent>[]>(
    () => Array.from(processor.getSurfaces().values()),
  );

  useEffect(() => {
    const sync = () => setSurfaces(Array.from(processor.getSurfaces().values()));
    const created = processor.onSurfaceCreated(sync);
    const deleted = processor.onSurfaceDeleted(sync);
    sync();
    return () => {
      created.unsubscribe();
      deleted.unsubscribe();
    };
  }, [processor]);

  if (error) {
    return <p className="text-red-400 text-xs">A2UI render error: {error}</p>;
  }
  return (
    <div className="a2ui-surfaces not-prose mt-3 w-full">
      {surfaces.map((surface) => (
        <A2uiSurface key={surface.id} surface={surface} />
      ))}
    </div>
  );
}
