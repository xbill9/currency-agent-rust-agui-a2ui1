import React from "react";

export function ExchangeRateCard({
  currencyFrom,
  currencyTo,
  rate,
  date,
  themeColor,
}: {
  currencyFrom: string;
  currencyTo: string;
  rate?: number;
  date?: string;
  themeColor: string;
}) {
  const handleSwap = () => {
    const event = new CustomEvent("a2ui-action", {
      detail: {
        name: "convert",
        params: { amount: 1, from: currencyTo, to: currencyFrom }
      }
    });
    window.dispatchEvent(event);
  };

  const handleTrend = () => {
    const event = new CustomEvent("a2ui-action", {
      detail: {
        name: "show_trends",
        params: { from: currencyFrom, to: currencyTo }
      }
    });
    window.dispatchEvent(event);
  };

  return (
    <div
      style={{
        borderLeft: `4px solid ${themeColor}`,
        boxShadow: `0 12px 40px 0 rgba(0, 0, 0, 0.5), inset 0 1px 1px 0 rgba(255, 255, 255, 0.1)`
      }}
      className="rounded-2xl mt-6 mb-4 max-w-md w-full bg-slate-900/95 border border-slate-800 overflow-hidden text-white backdrop-blur-lg animate-fade-in"
    >
      <div className="bg-slate-950/50 p-6 w-full">
        {/* Header */}
        <div className="flex items-center justify-between border-b border-slate-800/80 pb-4">
          <div>
            <span 
              style={{ backgroundColor: `${themeColor}15`, color: themeColor, borderColor: `${themeColor}30` }}
              className="px-2.5 py-1 rounded-md text-[10px] font-bold tracking-wider uppercase border"
            >
              Live Exchange Rate
            </span>
            <p className="text-slate-300 text-[10px] mt-2 font-medium tracking-wide">
              Frankfurter API • {date || new Date().toLocaleDateString()}
            </p>
          </div>
          <div className="text-3xl filter drop-shadow-[0_0_8px_rgba(255,255,255,0.2)]">💵</div>
        </div>

        {/* Conversion display */}
        <div className="mt-5 flex items-center justify-between bg-slate-950/80 p-4 rounded-xl border border-slate-800 shadow-inner">
          <div className="flex flex-col">
            <span className="text-[10px] text-slate-400 font-bold uppercase tracking-widest">Source</span>
            <span className="text-xl font-extrabold text-white mt-1">
              {currencyFrom}
            </span>
          </div>
          <div 
            style={{ color: themeColor, borderColor: `${themeColor}30`, backgroundColor: `${themeColor}05` }}
            className="w-9 h-9 rounded-full border flex items-center justify-center text-sm font-bold shadow-md"
          >
            ➔
          </div>
          <div className="flex flex-col text-right">
            <span className="text-[10px] text-slate-400 font-bold uppercase tracking-widest">Target</span>
            <span 
              style={{ color: themeColor }}
              className="text-xl font-extrabold mt-1"
            >
              {currencyTo}
            </span>
          </div>
        </div>

        {/* Big Rate Numbers */}
        <div className="mt-6 flex items-baseline justify-between border-b border-slate-800/80 pb-5">
          <div className="flex flex-col">
            <span className="text-[10px] text-slate-400 font-extrabold uppercase tracking-widest">Exchange Rate</span>
            <span 
              style={{ 
                color: "#ffffff", 
                textShadow: `0 0 20px ${themeColor}50, 0 0 4px ${themeColor}` 
              }}
              className="text-5xl font-black tracking-tighter mt-1.5 transition-all duration-300"
            >
              {rate ? rate.toFixed(4) : "—"}
            </span>
          </div>
          <div className="text-[12px] text-white bg-slate-950 px-3 py-1.5 rounded-lg border border-slate-800 font-semibold shadow-md">
            1 {currencyFrom} = <span style={{ color: themeColor }}>{rate ? rate.toFixed(4) : "?"}</span> {currencyTo}
          </div>
        </div>

        {/* Quick actions row */}
        <div className="mt-5 flex justify-end gap-2">
          <button
            onClick={handleSwap}
            className="px-4 py-2 rounded-lg text-[10px] font-extrabold tracking-wider uppercase transition-all duration-200 border border-slate-850 bg-slate-900/60 hover:bg-slate-850 hover:border-slate-750 text-slate-200 cursor-pointer shadow-sm"
          >
            🔄 Reverse
          </button>
          <button
            onClick={handleTrend}
            style={{ backgroundColor: `${themeColor}15`, color: themeColor, borderColor: `${themeColor}30` }}
            className="px-4 py-2 rounded-lg text-[10px] font-extrabold tracking-wider uppercase transition-all duration-200 hover:brightness-125 border cursor-pointer shadow-sm"
          >
            📈 View Trends
          </button>
        </div>
      </div>
    </div>
  );
}
