"use client";

// A2UI 1.0 renderer setup: the Basic catalog from @a2ui/web_core/v1_0 plus the
// currency-charts catalog (BarChart, LineChart). The catalog id and props must
// match currency_agent/custom_catalog.json on the agent side.

import React from "react";
import { z } from "zod";
import { renderMarkdown } from "@a2ui/markdown-it";
import {
  createComponentImplementation,
  type ReactCatalogComponent,
} from "@a2ui/react";
import {
  Catalog,
  basicCatalog,
  injectBasicCatalogStyles,
  setMarkdownRenderer,
} from "@a2ui/web_core/v1_0";

export const CHARTS_CATALOG_ID =
  "https://a2ui.org/catalog/currency-charts/v1/catalog.json";

let stylesReady = false;
export function ensureA2uiSetup() {
  if (stylesReady || typeof window === "undefined") return;
  injectBasicCatalogStyles();
  setMarkdownRenderer(renderMarkdown);
  stylesReady = true;
}

const chartProps = z.object({
  title: z.string().optional(),
  labels: z.array(z.string()),
  values: z.array(z.number()),
  color: z.string().optional(),
});
type ChartProps = z.infer<typeof chartProps>;

function BarChartView({ title, labels, values, color = "#10b981" }: ChartProps) {
  const minVal = Math.min(...values);
  const maxVal = Math.max(...values);
  const range = maxVal - minVal;
  const baseline = range > 1e-5 ? minVal - range * 0.15 : 0;
  const denom = maxVal - baseline > 0 ? maxVal - baseline : 1;
  return (
    <div className="bg-slate-900/50 border border-slate-700/50 rounded-xl p-4 mt-2 w-full">
      {title && <h5 className="m-0 mb-3 text-white text-sm font-semibold">{title}</h5>}
      <div className="flex items-end justify-around h-[140px] py-2 border-b border-slate-700">
        {labels.map((label, idx) => {
          const val = values[idx] ?? 0;
          const heightPercent = range > 0 ? ((val - baseline) / denom) * 100 : 100;
          return (
            <div key={idx} className="flex flex-col items-center flex-1 gap-2">
              <span className="text-[10px] text-white font-medium">{val.toFixed(4)}</span>
              <div
                style={{ height: `${heightPercent}%`, backgroundColor: color }}
                className="w-5 rounded-t transition-all duration-500 ease-out"
                title={`${val}`}
              />
              <span className="text-[10px] text-slate-400">{label}</span>
            </div>
          );
        })}
      </div>
    </div>
  );
}

function LineChartView({ title, labels, values, color = "#3b82f6" }: ChartProps) {
  const gradId = React.useId().replace(/:/g, "");
  if (labels.length < 2 || values.length < 2) {
    return <p className="text-red-400 text-xs">Not enough data points for line chart.</p>;
  }
  const minVal = Math.min(...values);
  const maxVal = Math.max(...values);
  const range = maxVal - minVal;
  const baseline = range > 1e-5 ? minVal - range * 0.15 : minVal * 0.9;
  const denom = maxVal - baseline > 0 ? maxVal - baseline : 1;
  const width = 350, height = 150, padX = 25, padY = 20;
  const points = values.map((val, idx) => ({
    x: padX + (idx / (values.length - 1)) * (width - 2 * padX),
    y: height - padY - ((val - baseline) / denom) * (height - 2 * padY),
  }));
  const lineD = points.map((p, i) => `${i ? "L" : "M"} ${p.x} ${p.y}`).join(" ");
  const areaD = `M ${points[0].x} ${height - padY} ${points.map((p) => `L ${p.x} ${p.y}`).join(" ")} L ${points[points.length - 1].x} ${height - padY} Z`;
  // Label at most ~8 points so long histories stay readable.
  const step = Math.max(1, Math.ceil(points.length / 8));
  return (
    <div className="bg-slate-900/50 border border-slate-700/50 rounded-xl p-4 mt-2 w-full">
      {title && <h5 className="m-0 mb-3 text-white text-sm font-semibold">{title}</h5>}
      <svg viewBox={`0 0 ${width} ${height}`} className="w-full h-[150px] overflow-visible">
        <defs>
          <linearGradient id={gradId} x1="0" y1="0" x2="0" y2="1">
            <stop offset="0%" stopColor={color} stopOpacity="0.25" />
            <stop offset="100%" stopColor={color} stopOpacity="0.0" />
          </linearGradient>
        </defs>
        {[0, 1, 2, 3].map((i) => {
          const gridY = padY + (i / 3) * (height - 2 * padY);
          return (
            <line key={i} x1={padX} y1={gridY} x2={width - padX} y2={gridY}
              stroke="rgba(255,255,255,0.05)" strokeDasharray="2,2" />
          );
        })}
        <path d={areaD} fill={`url(#${gradId})`} />
        <path d={lineD} stroke={color} strokeWidth="2.5" fill="none"
          strokeLinecap="round" strokeLinejoin="round" />
        {points.map((p, idx) => (
          <g key={idx}>
            <circle cx={p.x} cy={p.y} r="3" fill="#fff" stroke={color} strokeWidth="1.5" />
            {(idx % step === 0 || idx === points.length - 1) && (
              <>
                <text x={p.x} y={height - 5} textAnchor="middle" fill="rgba(255,255,255,0.4)" fontSize="8px">
                  {labels[idx]}
                </text>
                <text x={p.x} y={p.y - 7} textAnchor="middle" fill="#fff" fontSize="8px" fontWeight="500">
                  {values[idx].toFixed(4)}
                </text>
              </>
            )}
          </g>
        ))}
      </svg>
    </div>
  );
}

const BarChart = createComponentImplementation(
  { name: "BarChart", schema: chartProps },
  ({ props }) => <BarChartView {...(props as ChartProps)} />,
);

const LineChart = createComponentImplementation(
  { name: "LineChart", schema: chartProps },
  ({ props }) => <LineChartView {...(props as ChartProps)} />,
);

export const chartsCatalog = new Catalog<ReactCatalogComponent>(
  CHARTS_CATALOG_ID,
  "1.0",
  [BarChart, LineChart],
);

export const a2uiCatalogs = [basicCatalog, chartsCatalog];
