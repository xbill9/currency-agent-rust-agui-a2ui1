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
    }).passthrough(),
  },
  Text: {
    description: "A text element",
    props: z.object({
      value: z.string().optional(),
      text: z.string().optional(),
      variant: z.string().optional(),
    }).passthrough(),
  },
  Column: {
    description: "A column layout container",
    props: z.object({
      child: z.string().optional(),
      children: z.array(z.string()).optional(),
    }).passthrough(),
  },
  Row: {
    description: "A row layout container",
    props: z.object({
      children: z.array(z.string()).optional(),
    }).passthrough(),
  },
  Table: {
    description: "A table element",
    props: z.object({
      headers: z.array(z.any()).optional(),
      rows: z.array(z.any()).optional(),
    }).passthrough(),
  },
  BarChart: {
    description: "A vertical bar chart for showing rates",
    props: z.object({
      title: z.string().optional(),
      labels: z.array(z.any()).optional(),
      values: z.array(z.any()).optional(),
      color: z.string().optional(),
    }).passthrough(),
  },
  LineChart: {
    description: "An SVG line chart with gradients for showing historical rates",
    props: z.object({
      title: z.string().optional(),
      labels: z.array(z.any()).optional(),
      values: z.array(z.any()).optional(),
      color: z.string().optional(),
    }).passthrough(),
  },
  MultiLineChart: {
    description: "An SVG line chart with multiple series for showing historical rates of different currencies",
    props: z.object({
      title: z.string().optional(),
      labels: z.array(z.any()).optional(),
      series: z.array(z.any()).optional(),
    }).passthrough(),
  },
  Button: {
    description: "A clickable button element that triggers actions",
    props: z.object({
      child: z.string().optional(),
      primary: z.boolean().optional(),
      action: z.any().optional(),
    }).passthrough(),
  },
};

const customCatalogRenderers = {
  Card: (arg: any) => {
    const props = arg?.props || arg || {};
    const children = arg?.children;
    const cleanProps = { ...props };
    if (typeof cleanProps.children === "function") {
      delete cleanProps.children;
    }
    console.log("Card render props:", cleanProps);
    console.log("Card render children type:", typeof children, children);
    
    const isChildrenFn = typeof children === "function";
    const childIds: string[] = [];
    if (cleanProps.child) {
      childIds.push(cleanProps.child);
    }
    if (Array.isArray(cleanProps.children)) {
      cleanProps.children.forEach((id: string) => {
        if (!childIds.includes(id)) {
          childIds.push(id);
        }
      });
    }

    return (
      <div className="relative overflow-hidden bg-slate-800/90 border border-slate-600/60 rounded-2xl p-6 my-4 shadow-[0_25px_50px_-12px_rgba(0,0,0,0.8),_inset_0_1px_1px_0_rgba(255,255,255,0.15)] text-white w-full animate-fade-in hover:border-emerald-500/50 hover:shadow-[0_0_30px_rgba(16,185,129,0.15)] transition-all duration-300">
        {/* Glowing neon top stripe */}
        <div className="absolute top-0 left-0 right-0 h-[4px] bg-gradient-to-r from-emerald-500 via-teal-400 to-blue-500 opacity-100" />
        {cleanProps.title && (
          <h4 className="text-sm font-extrabold border-b border-slate-700/80 pb-3 mb-4 tracking-wide uppercase text-slate-100 flex items-center gap-2">
            <span className="text-emerald-400 text-base">💵</span> {cleanProps.title}
          </h4>
        )}
        <div className="space-y-4">
          {isChildrenFn ? (
            childIds.map((id: string) => {
              try {
                const res = children(id);
                console.log("Card children(id) result for", id, ":", res);
                return <React.Fragment key={id}>{res}</React.Fragment>;
              } catch (e) {
                console.error("Error rendering child id", id, "in Card:", e);
                return null;
              }
            })
          ) : (
            (typeof children === "object" && children !== null && "call" in children) || typeof children === "function" ? null : children
          )}
        </div>
      </div>
    );
  },
  Text: (arg: any) => {
    const props = arg?.props || arg || {};
    const textVal = props.value || props.text || "";
    const variant = props.variant || "body";
    if (variant === "h2") {
      return <h2 className="text-2xl font-black text-transparent bg-clip-text bg-gradient-to-r from-emerald-400 to-teal-300 my-2 filter drop-shadow-[0_2px_8px_rgba(16,185,129,0.3)]">{textVal}</h2>;
    }
    if (variant === "h3") {
      return <h3 className="text-lg font-extrabold text-white my-2 leading-relaxed tracking-wide">{textVal}</h3>;
    }
    if (variant === "h4") {
      return <h4 className="text-base font-bold text-slate-100 my-1 leading-relaxed">{textVal}</h4>;
    }
    if (variant === "caption") {
      return <p className="text-xs font-semibold text-slate-300 my-1.5 leading-relaxed tracking-wider">{textVal}</p>;
    }
    return <p className="text-sm font-medium text-slate-100 my-1 leading-relaxed">{textVal}</p>;
  },
  Column: (arg: any) => {
    const props = arg?.props || arg || {};
    const children = arg?.children;
    const cleanProps = { ...props };
    if (typeof cleanProps.children === "function") {
      delete cleanProps.children;
    }
    
    const isChildrenFn = typeof children === "function";
    const childIds: string[] = [];
    if (cleanProps.child) {
      childIds.push(cleanProps.child);
    }
    if (Array.isArray(cleanProps.children)) {
      cleanProps.children.forEach((id: string) => {
        if (!childIds.includes(id)) {
          childIds.push(id);
        }
      });
    }

    return (
      <div className="flex flex-col gap-4 w-full">
        {isChildrenFn ? (
          childIds.map((id: string) => {
            try {
              return <React.Fragment key={id}>{children(id)}</React.Fragment>;
            } catch (e) {
              console.error("Error rendering child id", id, "in Column:", e);
              return null;
            }
          })
        ) : (
          (typeof children === "object" && children !== null && "call" in children) || typeof children === "function" ? null : children
        )}
      </div>
    );
  },
  Row: (arg: any) => {
    const props = arg?.props || arg || {};
    const children = arg?.children;
    const cleanProps = { ...props };
    if (typeof cleanProps.children === "function") {
      delete cleanProps.children;
    }
    
    const isChildrenFn = typeof children === "function";
    const childIds: string[] = [];
    if (cleanProps.child) {
      childIds.push(cleanProps.child);
    }
    if (Array.isArray(cleanProps.children)) {
      cleanProps.children.forEach((id: string) => {
        if (!childIds.includes(id)) {
          childIds.push(id);
        }
      });
    }

    return (
      <div className="flex flex-row items-center gap-4 w-full flex-wrap">
        {isChildrenFn ? (
          childIds.map((id: string) => {
            try {
              return <React.Fragment key={id}>{children(id)}</React.Fragment>;
            } catch (e) {
              console.error("Error rendering child id", id, "in Row:", e);
              return null;
            }
          })
        ) : (
          (typeof children === "object" && children !== null && "call" in children) || typeof children === "function" ? null : children
        )}
      </div>
    );
  },
  Button: (arg: any) => {
    const props = arg?.props || arg || {};
    const children = arg?.children;
    const cleanProps = { ...props };
    if (typeof cleanProps.children === "function") {
      delete cleanProps.children;
    }
    const handleClick = () => {
      if (cleanProps.action) {
        const event = new CustomEvent("a2ui-action", { detail: cleanProps.action });
        window.dispatchEvent(event);
      }
    };
    const isChildrenFn = typeof children === "function";
    return (
      <button
        onClick={handleClick}
        className={`px-5 py-2.5 rounded-xl text-xs font-bold tracking-wider uppercase transition-all duration-200 cursor-pointer shadow-lg transform active:scale-95 ${
          cleanProps.primary
            ? "bg-gradient-to-r from-emerald-500 to-teal-400 hover:from-emerald-400 hover:to-teal-300 text-slate-950 shadow-emerald-500/20"
            : "bg-slate-700/80 hover:bg-slate-600 text-slate-100 border border-slate-600/60"
        }`}
      >
        {cleanProps.child && isChildrenFn ? (
          children(cleanProps.child)
        ) : (
          isChildrenFn || (typeof children === "object" && children !== null && "call" in children) ? "Click here" : (children || "Click here")
        )}
      </button>
    );
  },
  Table: (arg: any) => {
    const props = arg?.props || arg || {};
    const cleanProps = { ...props };
    if (typeof cleanProps.children === "function") {
      delete cleanProps.children;
    }
    let headers = Array.isArray(cleanProps.headers) ? cleanProps.headers : [];
    const rows = Array.isArray(cleanProps.rows) ? cleanProps.rows : [];
    if (headers.length === 0 && rows.length > 0 && !Array.isArray(rows[0]) && typeof rows[0] === "object" && !rows[0].cells) {
      headers = Object.keys(rows[0]);
    }
    return (
      <div className="bg-slate-750/90 border border-slate-600/60 rounded-xl p-5 my-4 overflow-x-auto w-full animate-fade-in shadow-[0_10px_30px_rgba(0,0,0,0.5)]">
        <table className="w-full text-xs text-left border-collapse">
          {headers.length > 0 && (
            <thead>
              <tr className="border-b border-slate-600/80 bg-slate-800/80">
                {headers.map((h: any, idx: number) => (
                  <th key={idx} className="p-3.5 text-slate-200 font-extrabold tracking-wider uppercase text-center border-r border-slate-700/50 last:border-r-0">
                    {typeof h === "string" ? h : h.value || ""}
                  </th>
                ))}
              </tr>
            </thead>
          )}
          <tbody>
            {rows.map((row: any, rIdx: number) => {
              let cells = [];
              if (Array.isArray(row)) {
                cells = row;
              } else if (row && typeof row === "object") {
                if (Array.isArray(row.cells)) {
                  cells = row.cells;
                } else {
                  cells = Object.values(row);
                }
              }
              return (
                <tr key={rIdx} className="border-b border-slate-700/30 last:border-0 hover:bg-slate-700/40 transition-colors">
                  {cells.map((cell: any, cIdx: number) => (
                    <td key={cIdx} className="p-3.5 text-white font-bold text-center border-r border-slate-700/20 last:border-r-0">
                      {typeof cell === "string" ? cell : (cell && typeof cell === "object" ? cell.value || JSON.stringify(cell) : String(cell))}
                    </td>
                  ))}
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>
    );
  },
  BarChart: (arg: any) => {
    const props = arg?.props || arg || {};
    const cleanProps = { ...props };
    if (typeof cleanProps.children === "function") {
      delete cleanProps.children;
    }
    const { title, labels = [], values = [], color = "#10b981" } = cleanProps;
    const parsedLabels = (Array.isArray(labels) ? labels : []).map((l: any) => String(l));
    const parsedValues = (Array.isArray(values) ? values : []).map((v: any) => {
      if (typeof v === "number") return v;
      const parsed = parseFloat(String(v));
      return isNaN(parsed) ? 0 : parsed;
    });

    if (parsedLabels.length === 0 || parsedValues.length === 0) {
      return <p className="text-slate-400 text-xs p-2">No data points for bar chart.</p>;
    }
    const minVal = Math.min(...parsedValues);
    const maxVal = Math.max(...parsedValues);
    const range = maxVal - minVal;
    const baseline = range > 1e-5 ? (minVal - range * 0.15) : 0;
    const denom = maxVal - baseline > 0 ? (maxVal - baseline) : 1;

    return (
      <div className="bg-slate-900/50 border border-slate-700/50 rounded-xl p-4 mt-2 w-full animate-fade-in">
        {title && <h5 className="m-0 mb-3 text-white text-sm font-semibold">{title}</h5>}
        <div className="flex items-end justify-around py-2 border-b border-slate-700">
          {parsedLabels.map((label: string, idx: number) => {
            const val = parsedValues[idx] ?? 0;
            const heightPercent = range > 0 ? (((val - baseline) / denom) * 100) : 100;
            return (
              <div key={idx} className="flex flex-col items-center flex-1 gap-1">
                <span className="text-[10px] text-white font-medium">{val.toFixed(4)}</span>
                <div className="h-[100px] w-full flex items-end justify-center">
                  <div
                    style={{
                      height: `${heightPercent}%`,
                      backgroundColor: color,
                    }}
                    className="w-5 rounded-t transition-all duration-500 ease-out animate-grow-up"
                    title={`${val}`}
                  />
                </div>
                <span className="text-[10px] text-slate-400">{label}</span>
              </div>
            );
          })}
        </div>
      </div>
    );
  },
  LineChart: (arg: any) => {
    const props = arg?.props || arg || {};
    const cleanProps = { ...props };
    if (typeof cleanProps.children === "function") {
      delete cleanProps.children;
    }
    const { title, labels, values, color = "#3b82f6" } = cleanProps;
    
    const parsedLabels = (Array.isArray(labels) ? labels : []).map((l: any) => String(l));
    const parsedValues = (Array.isArray(values) ? values : []).map((v: any) => {
      if (typeof v === "number") return v;
      const parsed = parseFloat(String(v));
      return isNaN(parsed) ? 0 : parsed;
    });

    // Smooth grace handle for empty or single data points to prevent crashes or ugly errors
    const effectiveLabels = [...parsedLabels];
    const effectiveValues = [...parsedValues];
    
    if (effectiveLabels.length === 0 || effectiveValues.length === 0) {
      return (
        <div className="bg-slate-750/90 border border-slate-600/60 rounded-xl p-5 mt-2 w-full flex flex-col items-center justify-center min-h-[160px] text-center animate-pulse">
          <span className="text-2xl mb-2 animate-bounce">📈</span>
          <p className="text-slate-300 text-xs font-semibold">Visualizing Trend Feed...</p>
          <p className="text-[10px] text-slate-500 mt-1">Waiting for rate streams to populate chart</p>
        </div>
      );
    }

    if (effectiveLabels.length === 1 && effectiveValues.length === 1) {
      // Duplicate to draw a beautiful flat baseline
      effectiveLabels.push(effectiveLabels[0]);
      effectiveValues.push(effectiveValues[0]);
    }

    const minVal = Math.min(...effectiveValues);
    const maxVal = Math.max(...effectiveValues);
    const range = maxVal - minVal;
    const baseline = range > 1e-5 ? (minVal - range * 0.15) : (minVal * 0.9);
    const denom = maxVal - baseline > 0 ? (maxVal - baseline) : 1;

    const width = 350;
    const height = 150;
    const padX = 25;
    const padY = 20;

    const points: { x: number; y: number }[] = [];
    effectiveLabels.forEach((label: string, idx: number) => {
      const val = effectiveValues[idx] || 0;
      const pctY = (val - baseline) / denom;
      const x = padX + (idx / (effectiveLabels.length - 1)) * (width - 2 * padX);
      const y = height - padY - pctY * (height - 2 * padY);
      points.push({ x, y });
    });

    let areaD = `M ${points[0].x} ${height - padY}`;
    points.forEach((p) => {
      areaD += ` L ${p.x} ${p.y}`;
    });
    areaD += ` L ${points[points.length - 1].x} ${height - padY} Z`;

    let lineD = `M ${points[0].x} ${points[0].y}`;
    for (let i = 1; i < points.length; i++) {
      lineD += ` L ${points[i].x} ${points[i].y}`;
    }

    const reactId = React.useId().replace(/:/g, "-");
    const gradId = `glow-grad-${reactId}`;

    return (
      <div className="bg-slate-750/90 border border-slate-600/60 rounded-xl p-5 mt-2 w-full shadow-[0_10px_30px_rgba(0,0,0,0.4)] animate-fade-in">
        {title && <h5 className="m-0 mb-4 text-white text-sm font-bold tracking-wide uppercase border-b border-slate-700/50 pb-2">{title}</h5>}
        <svg viewBox={`0 0 ${width} ${height}`} className="w-full h-[150px] overflow-visible">
          <defs>
            <linearGradient id={gradId} x1="0" y1="0" x2="0" y2="1">
              <stop offset="0%" stopColor={color} stopOpacity="0.35" />
              <stop offset="100%" stopColor={color} stopOpacity="0.0" />
            </linearGradient>
          </defs>

          {/* Grid lines */}
          {[0, 1, 2, 3].map((i) => {
            const gridY = padY + (i / 3) * (height - 2 * padY);
            return (
              <line
                key={i}
                x1={padX}
                y1={gridY}
                x2={width - padX}
                y2={gridY}
                stroke="rgba(255,255,255,0.08)"
                strokeDasharray="2,2"
              />
            );
          })}

          <path d={areaD} fill={`url(#${gradId})`} className="animate-fade-in" style={{ animationDelay: '0.4s' }} />

          <path
            d={lineD}
            stroke={color}
            strokeWidth="3"
            fill="none"
            strokeLinecap="round"
            strokeLinejoin="round"
            className="animate-draw-path"
          />

          {points.map((p, idx) => (
            <g key={idx} className="animate-fade-in" style={{ animationDelay: `${0.3 + idx * 0.15}s` }}>
              <circle
                cx={p.x}
                cy={p.y}
                r="5"
                fill="#fff"
                stroke={color}
                strokeWidth="2.5"
                className="filter drop-shadow-[0_0_4px_rgba(255,255,255,0.5)]"
              />
              {/* Only show dates for unique nodes */}
              {(idx === 0 || idx === points.length - 1 || effectiveLabels.length > 2) && (
                <text
                  x={p.x}
                  y={height - 4}
                  textAnchor="middle"
                  fill="rgba(255,255,255,0.8)"
                  fontSize="9px"
                  fontWeight="bold"
                >
                  {effectiveLabels[idx]}
                </text>
              )}
              <text
                x={p.x}
                y={p.y - 9}
                textAnchor="middle"
                fill="#fff"
                fontSize="9px"
                fontWeight="bold"
                className="filter drop-shadow-[0_1px_2px_rgba(0,0,0,0.8)]"
              >
                {(effectiveValues[idx] ?? 0).toFixed(4)}
              </text>
            </g>
          ))}
        </svg>
      </div>
    );
  },
  MultiLineChart: (arg: any) => {
    const props = arg?.props || arg || {};
    const cleanProps = { ...props };
    if (typeof cleanProps.children === "function") {
      delete cleanProps.children;
    }
    const { title, labels = [], series = [] } = cleanProps;
    
    const parsedLabels = (Array.isArray(labels) ? labels : []).map((l: any) => String(l));
    const parsedSeries = (Array.isArray(series) ? series : []).map((s: any) => {
      const name = s?.name || "Currency";
      const rawValues = Array.isArray(s?.values) ? s.values : [];
      const values = rawValues.map((v: any) => {
        if (typeof v === "number") return v;
        const parsed = parseFloat(String(v));
        return isNaN(parsed) ? 0 : parsed;
      });
      const color = s?.color || "#3b82f6";
      return { name, values, color };
    });

    if (parsedLabels.length === 0 || parsedSeries.length === 0) {
      return (
        <div className="bg-slate-750/90 border border-slate-600/60 rounded-xl p-5 mt-2 w-full flex flex-col items-center justify-center min-h-[160px] text-center animate-pulse">
          <span className="text-2xl mb-2 animate-bounce">📊</span>
          <p className="text-slate-300 text-xs font-semibold">Visualizing Multi-Trend Feed...</p>
          <p className="text-[10px] text-slate-500 mt-1">Waiting for rate streams to populate chart</p>
        </div>
      );
    }

    let allValues: number[] = [];
    parsedSeries.forEach(s => {
      allValues = allValues.concat(s.values);
    });
    if (allValues.length === 0) {
      allValues = [0];
    }
    const minVal = Math.min(...allValues);
    const maxVal = Math.max(...allValues);
    const range = maxVal - minVal;
    
    const baseline = range > 1e-5 ? (minVal - range * 0.15) : (minVal * 0.9);
    const denom = maxVal - baseline > 0 ? (maxVal - baseline) : 1;

    const width = 350;
    const height = 180;
    const padX = 35;
    const padY = 25;

    const reactId = React.useId().replace(/:/g, "-");

    return (
      <div className="bg-slate-750/90 border border-slate-600/60 rounded-xl p-5 mt-2 w-full shadow-[0_10px_30px_rgba(0,0,0,0.4)] animate-fade-in">
        {title && <h5 className="m-0 mb-4 text-white text-xs font-bold tracking-wide uppercase border-b border-slate-700/50 pb-2 flex items-center gap-2"><span className="text-emerald-400">📈</span> {title}</h5>}
        
        <div className="flex flex-row items-center gap-4 mb-4 flex-wrap">
          {parsedSeries.map((s, idx) => (
            <div key={idx} className="flex items-center gap-1.5 bg-slate-800/60 border border-slate-700/50 rounded-full px-2.5 py-1">
              <span className="w-2.5 h-2.5 rounded-full filter drop-shadow-[0_0_3px_rgba(255,255,255,0.2)]" style={{ backgroundColor: s.color }} />
              <span className="text-[10px] font-extrabold text-slate-200 tracking-wider uppercase">{s.name}</span>
            </div>
          ))}
        </div>

        <svg viewBox={`0 0 ${width} ${height}`} className="w-full h-[180px] overflow-visible">
          {[0, 1, 2, 3].map((i) => {
            const gridY = padY + (i / 3) * (height - 2 * padY);
            const gridVal = maxVal - (i / 3) * range;
            return (
              <g key={i}>
                <line
                  x1={padX}
                  y1={gridY}
                  x2={width - padX}
                  y2={gridY}
                  stroke="rgba(255,255,255,0.08)"
                  strokeDasharray="2,2"
                />
                <text
                  x={padX - 5}
                  y={gridY + 2.5}
                  textAnchor="end"
                  fill="rgba(255,255,255,0.5)"
                  fontSize="8px"
                  fontWeight="bold"
                >
                  {gridVal.toFixed(3)}
                </text>
              </g>
            );
          })}

          {parsedSeries.map((s, sIdx) => {
            const points: { x: number; y: number }[] = [];
            const effectiveLabels = [...parsedLabels];
            const effectiveValues = [...s.values];

            if (effectiveLabels.length === 1 && effectiveValues.length === 1) {
              effectiveLabels.push(effectiveLabels[0]);
              effectiveValues.push(effectiveValues[0]);
            }

            effectiveLabels.forEach((label: string, idx: number) => {
              const val = effectiveValues[idx] || 0;
              const pctY = (val - baseline) / denom;
              const x = padX + (idx / (effectiveLabels.length - 1)) * (width - 2 * padX);
              const y = height - padY - pctY * (height - 2 * padY);
              points.push({ x, y });
            });

            const gradId = `glow-grad-${reactId}-${sIdx}`;
            let areaD = `M ${points[0].x} ${height - padY}`;
            points.forEach((p) => {
              areaD += ` L ${p.x} ${p.y}`;
            });
            areaD += ` L ${points[points.length - 1].x} ${height - padY} Z`;

            let lineD = `M ${points[0].x} ${points[0].y}`;
            for (let i = 1; i < points.length; i++) {
              lineD += ` L ${points[i].x} ${points[i].y}`;
            }

            return (
              <g key={sIdx}>
                <defs>
                  <linearGradient id={gradId} x1="0" y1="0" x2="0" y2="1">
                    <stop offset="0%" stopColor={s.color} stopOpacity="0.2" />
                    <stop offset="100%" stopColor={s.color} stopOpacity="0.0" />
                  </linearGradient>
                </defs>

                <path d={areaD} fill={`url(#${gradId})`} className="animate-fade-in" style={{ animationDelay: '0.4s' }} />

                <path
                  d={lineD}
                  stroke={s.color}
                  strokeWidth="2.5"
                  fill="none"
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  className="animate-draw-path"
                />

                {points.map((p, idx) => (
                  <g key={idx} className="group cursor-pointer">
                    <circle
                      cx={p.x}
                      cy={p.y}
                      r="4"
                      fill="#fff"
                      stroke={s.color}
                      strokeWidth="2"
                      className="transition-all duration-200 hover:r-6"
                    />
                    <text
                      x={p.x}
                      y={p.y - 7}
                      textAnchor="middle"
                      fill="#fff"
                      fontSize="8px"
                      fontWeight="bold"
                      className="opacity-80 group-hover:opacity-100 transition-opacity pointer-events-none filter drop-shadow-[0_1px_2px_rgba(0,0,0,0.8)]"
                    >
                      {(effectiveValues[idx] ?? 0).toFixed(4)}
                    </text>
                    {sIdx === 0 && (idx === 0 || idx === points.length - 1 || effectiveLabels.length > 2) && (
                      <text
                        x={p.x}
                        y={height - 5}
                        textAnchor="middle"
                        fill="rgba(255,255,255,0.7)"
                        fontSize="9px"
                        fontWeight="bold"
                      >
                        {effectiveLabels[idx]}
                      </text>
                    )}
                  </g>
                ))}
              </g>
            );
          })}
        </svg>
      </div>
    );
  },
};

export const customCatalog = createCatalog(customCatalogDefinitions as any, customCatalogRenderers as any, {
  catalogId: "https://a2ui.org/specification/v0_9/basic_catalog.json"
});



