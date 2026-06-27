"use client";

import React from "react";
import { CopilotKit } from "@copilotkit/react-core/v2";
import { A2UIProvider } from "@copilotkit/a2ui-renderer";
import { customCatalog } from "./A2UICustomCatalog";
import "@copilotkit/react-core/v2/styles.css";

export default function CopilotProvider({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <CopilotKit runtimeUrl="/api/copilotkit" agent="currency_agent">
      <A2UIProvider catalog={customCatalog}>
        {children}
      </A2UIProvider>
    </CopilotKit>
  );
}

