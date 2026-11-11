"use client";

import React from "react";
import { CopilotKit } from "@copilotkit/react-core/v2";
import "@copilotkit/react-core/v2/styles.css";

export default function CopilotProvider({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <CopilotKit runtimeUrl="/api/copilotkit" agent="currency_agent">
      {children}
    </CopilotKit>
  );
}

