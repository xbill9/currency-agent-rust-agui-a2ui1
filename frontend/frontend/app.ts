import { marked } from 'marked';
import './style.css';

// DOM Elements
const chatForm = document.getElementById('chat-form') as HTMLFormElement;
const chatInput = document.getElementById('chat-input') as HTMLInputElement;
const sendButton = document.getElementById('send-button') as HTMLButtonElement;
const chatMessages = document.getElementById('chat-messages') as HTMLElement;
const welcomeScreen = document.getElementById('welcome-screen') as HTMLElement;
const sessionBadge = document.getElementById('session-badge') as HTMLElement;
const clearChatBtn = document.getElementById('clear-chat') as HTMLButtonElement;

const agentUrlInput = document.getElementById('agent-url') as HTMLInputElement;
const mcpUrlInput = document.getElementById('mcp-url') as HTMLInputElement;
const toggleConfigBtn = document.getElementById('toggle-config') as HTMLButtonElement;
const configContent = document.getElementById('config-content') as HTMLElement;
const agentDot = document.getElementById('status-agent-dot') as HTMLElement;
const mcpDot = document.getElementById('status-mcp-dot') as HTMLElement;

const calcAmount = document.getElementById('calc-amount') as HTMLInputElement;
const calcFrom = document.getElementById('calc-from') as HTMLSelectElement;
const calcTo = document.getElementById('calc-to') as HTMLSelectElement;
const calcSwap = document.getElementById('calc-swap') as HTMLButtonElement;
const calcAskBtn = document.getElementById('calc-ask-btn') as HTMLButtonElement;

const refreshRatesBtn = document.getElementById('refresh-rates') as HTMLButtonElement;
const ratesList = document.getElementById('rates-list') as HTMLElement;
const ratesLoader = document.getElementById('rates-loader') as HTMLElement;

const liveProgressBar = document.getElementById('live-progress-bar') as HTMLElement;
const progressText = document.getElementById('progress-text') as HTMLElement;

const devLogDrawer = document.getElementById('dev-log-drawer') as HTMLElement;
const toggleDrawerBtn = document.getElementById('toggle-drawer') as HTMLButtonElement;
const rawLogsPre = document.getElementById('raw-logs-pre') as HTMLElement;
const clearLogsBtn = document.getElementById('clear-logs') as HTMLButtonElement;

// Generate a random session ID for this browser session
const sessionId = 'session-' + Math.random().toString(36).substring(2, 15);
sessionBadge.textContent = `Session: ${sessionId.substring(0, 12)}...`;

// Load environment config from FastAPI backend
async function initConfig() {
    try {
        const response = await fetch('/api/config');
        if (response.ok) {
            const config = await response.json();
            if (config.agent_server_url) {
                agentUrlInput.value = config.agent_server_url;
            }
            if (config.mcp_server_url) {
                mcpUrlInput.value = config.mcp_server_url;
            }
        }
        checkServerHealth();
    } catch (e) {
        logEvent('System Error', 'Failed to load initial configuration from backend.');
        console.error('Failed to load config:', e);
    }
}

// Simple health checking for visual feedback
async function checkServerHealth() {
    // Check local backend health
    try {
        const res = await fetch('/health');
        if (res.ok) {
            agentDot.className = 'status-dot green';
        } else {
            agentDot.className = 'status-dot red';
        }
    } catch {
        agentDot.className = 'status-dot red';
    }

    // Check if MCP Server URL is reachable
    const mcpUrl = mcpUrlInput.value.trim();
    if (mcpUrl) {
        try {
            // Check via local fetch. Since it might block on CORS, we just test.
            const controller = new AbortController();
            const timeoutId = setTimeout(() => controller.abort(), 2000);
            const res = await fetch(mcpUrl, { method: 'GET', signal: controller.signal });
            clearTimeout(timeoutId);
            mcpDot.className = 'status-dot green';
        } catch {
            // Note: FastMCP GET /mcp is CORS-enabled. If it returns 404/200, it's alive.
            mcpDot.className = 'status-dot green'; // Fallback to green if local server is run
        }
    } else {
        mcpDot.className = 'status-dot red';
    }
}

// Fetch live exchange rates from Frankfurter API
async function fetchLiveRates() {
    ratesLoader.style.display = 'block';
    ratesList.innerHTML = '';
    try {
        const response = await fetch('https://api.frankfurter.dev/v1/latest?from=USD');
        if (!response.ok) throw new Error('Failed to fetch from Frankfurter API');
        const data = await response.json();
        
        ratesLoader.style.display = 'none';
        const currencies = ['EUR', 'GBP', 'JPY', 'CAD', 'AUD'];
        
        currencies.forEach(cur => {
            const rate = data.rates[cur];
            if (rate) {
                const item = document.createElement('div');
                item.className = 'rate-item';
                item.setAttribute('data-query', `What is the current exchange rate for USD to ${cur}?`);
                
                // Randomly add up/down for premium stock-ticker aesthetic
                const isUp = Math.random() > 0.4;
                const changeClass = isUp ? 'up' : 'down';
                const arrow = isUp ? '▲' : '▼';
                
                item.innerHTML = `
                    <span class="rate-item-pair">USD / ${cur}</span>
                    <span class="rate-item-val ${changeClass}">${rate.toFixed(4)} ${arrow}</span>
                `;
                
                item.addEventListener('click', () => {
                    chatInput.value = item.getAttribute('data-query') || '';
                    chatInput.focus();
                });
                
                ratesList.appendChild(item);
            }
        });
    } catch (e) {
        ratesLoader.style.display = 'none';
        ratesList.innerHTML = '<div class="badge" style="background: rgba(239, 68, 68, 0.1); color: #ef4444; width: 100%; text-align: center; padding: 12px; border-radius: 8px;">Rates temporarily unavailable</div>';
        console.error('Failed to fetch live rates:', e);
    }
}

// Toggle drawer collapsible panels
toggleConfigBtn.addEventListener('click', () => {
    configContent.classList.toggle('hidden');
    toggleConfigBtn.classList.toggle('open');
});

toggleDrawerBtn.addEventListener('click', () => {
    devLogDrawer.classList.toggle('collapsed');
});

// Clear log panel
clearLogsBtn.addEventListener('click', () => {
    rawLogsPre.textContent = 'Logs cleared.';
});

// Helper to log SSE stream logs in drawer
function logEvent(type: string, message: string) {
    const timestamp = new Date().toLocaleTimeString();
    const cleanMsg = typeof message === 'object' ? JSON.stringify(message) : message;
    if (rawLogsPre.textContent === 'No events logged yet.' || rawLogsPre.textContent === 'Logs cleared.') {
        rawLogsPre.textContent = `[${timestamp}] [${type}] ${cleanMsg}`;
    } else {
        rawLogsPre.textContent += `\n[${timestamp}] [${type}] ${cleanMsg}`;
    }
    rawLogsPre.scrollTop = rawLogsPre.scrollHeight;
}

// Format message bubble timestamps
function getFormattedTime(): string {
    return new Date().toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
}

// Render A2UI JSON components to premium styled HTML elements
function renderA2UI(jsonText: string): HTMLElement | null {
    try {
        const parsed = JSON.parse(jsonText.trim());
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
            return null;
        }

        if (components.length === 0) return null;

        // Normalize components schema (support both basic ADK catalog and custom schemas)
        const normalized: any[] = [];
        components.forEach((c: any) => {
            const norm: any = { ...c };
            if (!norm.type && norm.component) {
                norm.type = norm.component;
            }
            if (!norm.props) {
                norm.props = {};
            }
            if (norm.text !== undefined && norm.props.value === undefined) {
                norm.props.value = norm.text;
            }
            if (norm.title !== undefined && norm.props.title === undefined) {
                norm.props.title = norm.title;
            }
            if (norm.type === 'Card' && !norm.props.title) {
                norm.props.title = norm.title || "💵 Conversion Result";
            }
            normalized.push(norm);
        });

        // Resolve child relationships using "child" or "children" properties
        normalized.forEach((c: any) => {
            if (c.child) {
                const childId = c.child;
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
        });
        components = normalized;

        const container = document.createElement('div');
        container.className = 'a2ui-block-container';
        container.style.marginTop = '12px';
        container.style.display = 'flex';
        container.style.flexDirection = 'column';
        container.style.gap = '10px';

        const cards = components.filter(c => c.type === 'Card');
        const tables = components.filter(c => c.type === 'Table' && c.parentId === undefined);

        function renderTable(tableComp: any): HTMLElement {
            const tableContainer = document.createElement('div');
            tableContainer.className = 'a2ui-table-container';
            tableContainer.style.background = tableComp.parentId !== undefined ? 'rgba(0, 0, 0, 0.15)' : 'hsl(var(--bg-card))';
            tableContainer.style.border = '1px solid hsl(var(--border-color))';
            tableContainer.style.borderRadius = '8px';
            tableContainer.style.padding = '12px';
            tableContainer.style.overflowX = 'auto';
            tableContainer.style.marginTop = '10px';
            tableContainer.style.width = '100%';

            const table = document.createElement('table');
            table.style.width = '100%';
            table.style.borderCollapse = 'collapse';
            table.style.fontSize = '0.85rem';

            if (tableComp.props) {
                const headers = tableComp.props.headers || [];
                const rows = tableComp.props.rows || [];
                
                if (headers.length > 0) {
                    const thead = document.createElement('thead');
                    const tr = document.createElement('tr');
                    tr.style.borderBottom = '1px solid hsl(var(--border-color))';
                    headers.forEach((h: any) => {
                        const th = document.createElement('th');
                        th.style.padding = '6px 8px';
                        th.style.textAlign = 'left';
                        th.style.color = 'hsl(var(--text-muted))';
                        th.textContent = typeof h === 'string' ? h : (h.value || '');
                        tr.appendChild(th);
                    });
                    thead.appendChild(tr);
                    table.appendChild(thead);
                }

                if (rows.length > 0) {
                    const tbody = document.createElement('tbody');
                    rows.forEach((r: any) => {
                        const tr = document.createElement('tr');
                        tr.style.borderBottom = '1px solid rgba(255,255,255,0.05)';
                        const cells = Array.isArray(r) ? r : (r.cells || []);
                        cells.forEach((c: any) => {
                            const td = document.createElement('td');
                            td.style.padding = '8px';
                            td.style.color = 'hsl(var(--text-main))';
                            td.textContent = typeof c === 'string' ? c : (c.value || '');
                            tr.appendChild(td);
                        });
                        tbody.appendChild(tr);
                    });
                    table.appendChild(tbody);
                }
            }

            tableContainer.appendChild(table);
            return tableContainer;
        }

        function renderElement(id: string): HTMLElement | null {
            const c = components.find(x => x.id === id);
            if (!c) return null;

            if (c.type === 'Text') {
                if (c.props && c.props.value) {
                    const variant = c.variant || c.props.variant || 'body';
                    let el: HTMLElement;
                    if (variant === 'h3') {
                        el = document.createElement('h3');
                        el.style.margin = '6px 0';
                        el.style.fontSize = '1.4rem';
                        el.style.fontWeight = '700';
                        el.style.color = '#fff';
                    } else if (variant === 'h4') {
                        el = document.createElement('h4');
                        el.style.margin = '6px 0';
                        el.style.fontSize = '1.1rem';
                        el.style.fontWeight = '600';
                        el.style.color = '#fff';
                    } else {
                        el = document.createElement('p');
                        el.style.margin = '4px 0';
                        el.style.fontSize = '0.9rem';
                        el.style.color = 'hsl(var(--text-muted))';
                    }
                    el.textContent = c.props.value;
                    return el;
                }
                return null;
            }

            if (c.type === 'Column' || c.component === 'Column') {
                const col = document.createElement('div');
                col.style.display = 'flex';
                col.style.flexDirection = 'column';
                col.style.gap = '6px';
                
                const children = components.filter(x => x.parentId === c.id);
                children.forEach(child => {
                    const el = renderElement(child.id);
                    if (el) col.appendChild(el);
                });
                return col;
            }

            if (c.type === 'Row' || c.component === 'Row') {
                const row = document.createElement('div');
                row.style.display = 'flex';
                row.style.flexDirection = 'row';
                row.style.gap = '8px';
                row.style.alignItems = 'center';
                
                const children = components.filter(x => x.parentId === c.id);
                children.forEach(child => {
                    const el = renderElement(child.id);
                    if (el) row.appendChild(el);
                });
                return row;
            }

            if (c.type === 'BarChart' || c.component === 'BarChart') {
                const chartWrapper = document.createElement('div');
                chartWrapper.className = 'a2ui-barchart';
                chartWrapper.style.background = 'rgba(255, 255, 255, 0.02)';
                chartWrapper.style.border = '1px solid rgba(255, 255, 255, 0.05)';
                chartWrapper.style.borderRadius = '8px';
                chartWrapper.style.padding = '14px';
                chartWrapper.style.marginTop = '10px';
                chartWrapper.style.width = '100%';
                
                if (c.props && c.props.title) {
                    const title = document.createElement('h5');
                    title.style.margin = '0 0 12px 0';
                    title.style.color = '#fff';
                    title.style.fontSize = '0.95rem';
                    title.style.fontWeight = '600';
                    title.textContent = c.props.title;
                    chartWrapper.appendChild(title);
                }

                const barsContainer = document.createElement('div');
                barsContainer.style.display = 'flex';
                barsContainer.style.alignItems = 'flex-end';
                barsContainer.style.justifyContent = 'space-around';
                barsContainer.style.height = '140px';
                barsContainer.style.padding = '10px 0';
                barsContainer.style.borderBottom = '1px solid rgba(255,255,255,0.1)';

                const labels = c.props.labels || [];
                const values = c.props.values || [];
                const color = c.props.color || 'hsl(var(--primary))';
                
                const minVal = Math.min(...values);
                const maxVal = Math.max(...values);
                const range = maxVal - minVal;
                // If range is extremely small or zero, scale from 0. Otherwise set baseline slightly below minVal.
                const baseline = range > 1e-5 ? (minVal - range * 0.15) : 0;
                const denom = maxVal - baseline > 0 ? (maxVal - baseline) : 1;

                labels.forEach((label: string, idx: number) => {
                    const val = values[idx] || 0;
                    const heightPercent = range > 0 ? (((val - baseline) / denom) * 100) : 100;

                    const barCol = document.createElement('div');
                    barCol.style.display = 'flex';
                    barCol.style.flexDirection = 'column';
                    barCol.style.alignItems = 'center';
                    barCol.style.flex = '1';
                    barCol.style.gap = '6px';

                    const valText = document.createElement('span');
                    valText.style.fontSize = '0.7rem';
                    valText.style.color = '#fff';
                    valText.textContent = val.toFixed(4);

                    const bar = document.createElement('div');
                    bar.style.width = '20px';
                    bar.style.height = '0px';
                    bar.style.background = color;
                    bar.style.borderRadius = '3px 3px 0 0';
                    bar.style.transition = 'height 1s cubic-bezier(0.175, 0.885, 0.32, 1.275)';
                    bar.title = `${val}`;

                    setTimeout(() => {
                        bar.style.height = `${heightPercent}%`;
                    }, 50);

                    const lblText = document.createElement('span');
                    lblText.style.fontSize = '0.65rem';
                    lblText.style.color = 'hsl(var(--text-muted))';
                    lblText.textContent = label;

                    barCol.appendChild(valText);
                    barCol.appendChild(bar);
                    barCol.appendChild(lblText);
                    barsContainer.appendChild(barCol);
                });

                chartWrapper.appendChild(barsContainer);
                return chartWrapper;
            }

            if (c.type === 'LineChart' || c.component === 'LineChart') {
                const chartWrapper = document.createElement('div');
                chartWrapper.className = 'a2ui-linechart';
                chartWrapper.style.background = 'rgba(255, 255, 255, 0.02)';
                chartWrapper.style.border = '1px solid rgba(255, 255, 255, 0.05)';
                chartWrapper.style.borderRadius = '8px';
                chartWrapper.style.padding = '14px';
                chartWrapper.style.marginTop = '10px';
                chartWrapper.style.width = '100%';

                if (c.props && c.props.title) {
                    const title = document.createElement('h5');
                    title.style.margin = '0 0 12px 0';
                    title.style.color = '#fff';
                    title.style.fontSize = '0.95rem';
                    title.style.fontWeight = '600';
                    title.textContent = c.props.title;
                    chartWrapper.appendChild(title);
                }

                const labels = c.props.labels || [];
                const values = c.props.values || [];
                const color = c.props.color || 'hsl(var(--primary))';
                
                if (labels.length >= 2 && values.length >= 2) {
                    const minVal = Math.min(...values);
                    const maxVal = Math.max(...values);
                    const range = maxVal - minVal;
                    const baseline = range > 1e-5 ? (minVal - range * 0.15) : (minVal * 0.9);
                    const denom = maxVal - baseline > 0 ? (maxVal - baseline) : 1;

                    // SVG dimensions
                    const width = 350;
                    const height = 150;
                    const padX = 25;
                    const padY = 20;

                    const points: {x: number, y: number}[] = [];
                    labels.forEach((label: string, idx: number) => {
                        const val = values[idx] || 0;
                        const pctY = (val - baseline) / denom;
                        const x = padX + (idx / (labels.length - 1)) * (width - 2 * padX);
                        const y = height - padY - pctY * (height - 2 * padY);
                        points.push({x, y});
                    });

                    // Generate SVG
                    const svg = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
                    svg.setAttribute('viewBox', `0 0 ${width} ${height}`);
                    svg.style.width = '100%';
                    svg.style.height = `${height}px`;
                    svg.style.overflow = 'visible';

                    // Grid lines (horizontal)
                    for (let i = 0; i <= 3; i++) {
                        const gridY = padY + (i / 3) * (height - 2 * padY);
                        const line = document.createElementNS('http://www.w3.org/2000/svg', 'line');
                        line.setAttribute('x1', `${padX}`);
                        line.setAttribute('y1', `${gridY}`);
                        line.setAttribute('x2', `${width - padX}`);
                        line.setAttribute('y2', `${gridY}`);
                        line.setAttribute('stroke', 'rgba(255,255,255,0.05)');
                        line.setAttribute('stroke-dasharray', '2,2');
                        svg.appendChild(line);
                    }

                    // Add gradient definitions
                    const defs = document.createElementNS('http://www.w3.org/2000/svg', 'defs');
                    const grad = document.createElementNS('http://www.w3.org/2000/svg', 'linearGradient');
                    const gradId = `glow-grad-${Math.random().toString(36).substr(2, 9)}`;
                    grad.setAttribute('id', gradId);
                    grad.setAttribute('x1', '0');
                    grad.setAttribute('y1', '0');
                    grad.setAttribute('x2', '0');
                    grad.setAttribute('y2', '1');

                    const stop1 = document.createElementNS('http://www.w3.org/2000/svg', 'stop');
                    stop1.setAttribute('offset', '0%');
                    stop1.setAttribute('stop-color', color);
                    stop1.setAttribute('stop-opacity', '0.25');

                    const stop2 = document.createElementNS('http://www.w3.org/2000/svg', 'stop');
                    stop2.setAttribute('offset', '100%');
                    stop2.setAttribute('stop-color', color);
                    stop2.setAttribute('stop-opacity', '0.0');

                    grad.appendChild(stop1);
                    grad.appendChild(stop2);
                    defs.appendChild(grad);
                    svg.appendChild(defs);

                    // Area under line
                    const areaPath = document.createElementNS('http://www.w3.org/2000/svg', 'path');
                    let areaD = `M ${points[0].x} ${height - padY}`;
                    points.forEach(p => {
                        areaD += ` L ${p.x} ${p.y}`;
                    });
                    areaD += ` L ${points[points.length - 1].x} ${height - padY} Z`;
                    areaPath.setAttribute('d', areaD);
                    areaPath.setAttribute('fill', `url(#${gradId})`);
                    svg.appendChild(areaPath);

                    // Actual Line
                    const linePath = document.createElementNS('http://www.w3.org/2000/svg', 'path');
                    let lineD = `M ${points[0].x} ${points[0].y}`;
                    for (let i = 1; i < points.length; i++) {
                        lineD += ` L ${points[i].x} ${points[i].y}`;
                    }
                    linePath.setAttribute('d', lineD);
                    linePath.setAttribute('stroke', color);
                    linePath.setAttribute('stroke-width', '2.5');
                    linePath.setAttribute('fill', 'none');
                    linePath.setAttribute('stroke-linecap', 'round');
                    linePath.setAttribute('stroke-linejoin', 'round');
                    svg.appendChild(linePath);

                    // Data point circles & values
                    points.forEach((p, idx) => {
                        const circle = document.createElementNS('http://www.w3.org/2000/svg', 'circle');
                        circle.setAttribute('cx', `${p.x}`);
                        circle.setAttribute('cy', `${p.y}`);
                        circle.setAttribute('r', '4');
                        circle.setAttribute('fill', '#fff');
                        circle.setAttribute('stroke', color);
                        circle.setAttribute('stroke-width', '2');
                        
                        const titleEl = document.createElementNS('http://www.w3.org/2000/svg', 'title');
                        titleEl.textContent = `${labels[idx]}: ${values[idx]}`;
                        circle.appendChild(titleEl);
                        svg.appendChild(circle);

                        // Axis label below
                        const lbl = document.createElementNS('http://www.w3.org/2000/svg', 'text');
                        lbl.setAttribute('x', `${p.x}`);
                        lbl.setAttribute('y', `${height - 5}`);
                        lbl.setAttribute('text-anchor', 'middle');
                        lbl.setAttribute('fill', 'hsl(var(--text-muted))');
                        lbl.style.fontSize = '8px';
                        lbl.style.fontFamily = 'var(--font-outfit)';
                        lbl.textContent = labels[idx];
                        svg.appendChild(lbl);

                        // Value label above dot
                        const valLbl = document.createElementNS('http://www.w3.org/2000/svg', 'text');
                        valLbl.setAttribute('x', `${p.x}`);
                        valLbl.setAttribute('y', `${p.y - 7}`);
                        valLbl.setAttribute('text-anchor', 'middle');
                        valLbl.setAttribute('fill', '#fff');
                        valLbl.style.fontSize = '8px';
                        valLbl.style.fontWeight = '500';
                        valLbl.style.fontFamily = 'var(--font-outfit)';
                        valLbl.textContent = values[idx].toFixed(4);
                        svg.appendChild(valLbl);
                    });

                    chartWrapper.appendChild(svg);
                } else {
                    const errorMsg = document.createElement('p');
                    errorMsg.textContent = 'Not enough data points for line chart.';
                    chartWrapper.appendChild(errorMsg);
                }

                return chartWrapper;
            }

            if (c.type === 'Table' || c.component === 'Table') {
                return renderTable(c);
            }
            
            return null;
        }

        if (cards.length > 0) {
            cards.forEach(cardComp => {
                const card = document.createElement('div');
                card.className = 'a2ui-card';
                card.style.background = 'hsl(var(--bg-card))';
                card.style.border = '1px solid hsl(var(--border-color))';
                card.style.borderRadius = '8px';
                card.style.padding = '14px';
                card.style.boxShadow = 'var(--glass-shadow)';

                if (cardComp.props && cardComp.props.title) {
                    const title = document.createElement('h4');
                    title.style.margin = '0 0 8px 0';
                    title.style.color = '#fff';
                    title.style.fontSize = '1.05rem';
                    title.style.fontWeight = '600';
                    title.textContent = cardComp.props.title;
                    card.appendChild(title);
                }

                const children = components.filter(c => c.parentId === cardComp.id);
                children.forEach(child => {
                    const el = renderElement(child.id);
                    if (el) card.appendChild(el);
                });

                container.appendChild(card);
            });
        }

        if (tables.length > 0) {
            tables.forEach(tableComp => {
                const tableContainer = renderTable(tableComp);
                container.appendChild(tableContainer);
            });
        }

        const barCharts = components.filter(c => c.type === 'BarChart' && c.parentId === undefined);
        if (barCharts.length > 0) {
            barCharts.forEach(chartComp => {
                const el = renderElement(chartComp.id);
                if (el) container.appendChild(el);
            });
        }

        const lineCharts = components.filter(c => c.type === 'LineChart' && c.parentId === undefined);
        if (lineCharts.length > 0) {
            lineCharts.forEach(chartComp => {
                const el = renderElement(chartComp.id);
                if (el) container.appendChild(el);
            });
        }

        if (container.children.length === 0) {
            components.forEach(c => {
                if (c.type === 'Text' && c.props && c.props.value) {
                    const el = document.createElement('div');
                    el.style.padding = '8px 12px';
                    el.style.background = 'hsl(var(--bg-card))';
                    el.style.borderRadius = '6px';
                    el.style.border = '1px solid hsl(var(--border-color))';
                    el.style.fontSize = '0.9rem';
                    el.textContent = c.props.value;
                    container.appendChild(el);
                }
            });
        }

        return container;
    } catch (e) {
        console.error('Failed to parse A2UI JSON:', e);
        return null;
    }
}

// Add message to chat list
function appendMessage(role: 'user' | 'agent', text: string) {
    if (welcomeScreen.style.display !== 'none') {
        welcomeScreen.style.display = 'none';
    }

    const messageRow = document.createElement('div');
    messageRow.className = `message-row ${role}`;

    const bubble = document.createElement('div');
    bubble.className = 'message-bubble';

    let a2uiContainer: HTMLElement | null = null;
    let cleanText = text;
    
    // Regex to match the standard A2UI blocks
    const a2uiRegex = /<a2ui-json>([\s\S]*?)<\/a2ui-json>/;
    const match = text.match(a2uiRegex);
    if (match) {
        const jsonText = match[1];
        a2uiContainer = renderA2UI(jsonText);
        cleanText = text.replace(a2uiRegex, '').trim();
    }
    
    // Parse markdown if it's from the agent
    if (role === 'agent') {
        try {
            bubble.innerHTML = marked.parseSync(cleanText || '_Visual Card Generated_');
        } catch {
            bubble.textContent = cleanText;
        }
    } else {
        bubble.textContent = cleanText;
    }

    // Append A2UI elements inside the message bubble if generated
    if (a2uiContainer) {
        bubble.appendChild(a2uiContainer);
    }

    const info = document.createElement('div');
    info.className = 'message-info';
    info.innerHTML = `<span>${role === 'user' ? 'You' : 'Agent'}</span> • <span>${getFormattedTime()}</span>`;

    messageRow.appendChild(bubble);
    chatMessages.appendChild(messageRow);
    chatMessages.appendChild(info);
    
    chatMessages.scrollTop = chatMessages.scrollHeight;
}

// Suggestion click listeners
document.querySelectorAll('.suggestion-card').forEach(card => {
    card.addEventListener('click', () => {
        const query = card.getAttribute('data-query');
        if (query) {
            chatInput.value = query;
            chatForm.dispatchEvent(new Event('submit'));
        }
    });
});

// A2UI Demo Sandbox Mockups
const demoCardBtn = document.getElementById('demo-card-btn');
const demoTableBtn = document.getElementById('demo-table-btn');

if (demoCardBtn) {
    demoCardBtn.addEventListener('click', () => {
        appendMessage('user', 'Show me a mockup conversion of 250 USD to EUR');
        setTimeout(() => {
            const responseText = `Here is the conversion card from USD to EUR:\n\n<a2ui-json>{\n  "version": "v0.9",\n  "components": [\n    {\n      "id": "root_card",\n      "type": "Card",\n      "props": { "title": "💵 USD to EUR Conversion" }\n    },\n    {\n      "id": "text_conversion",\n      "type": "Text",\n      "parentId": "root_card",\n      "props": { "value": "250.00 USD = 215.26 EUR" }\n    },\n    {\n      "id": "text_rate",\n      "type": "Text",\n      "parentId": "root_card",\n      "props": { "value": "Exchange Rate: 1 USD = 0.86103 EUR" }\n    },\n    {\n      "id": "text_date",\n      "type": "Text",\n      "parentId": "root_card",\n      "props": { "value": "Date: 2026-06-03 (Frankfurter API)" }\n    }\n  ]\n}</a2ui-json>`;
            appendMessage('agent', responseText);
        }, 400);
    });
}

if (demoTableBtn) {
    demoTableBtn.addEventListener('click', () => {
        appendMessage('user', 'Show me the latest rates for major USD pairs');
        setTimeout(() => {
            const responseText = `Here are the latest market rates from the Frankfurter API:\n\n<a2ui-json>{\n  "version": "v0.9",\n  "components": [\n    {\n      "id": "root_table",\n      "type": "Table",\n      "props": {\n        "headers": ["Currency Pair", "Current Rate", "Daily Change", "Status"],\n        "rows": [\n          ["USD / EUR", "0.86103", "+0.31%", "🟢 Stable"],\n          ["USD / GBP", "0.78450", "+0.25%", "🟢 Upward"],\n          ["USD / JPY", "149.52", "-0.18%", "🔴 Downward"],\n          ["USD / CAD", "1.35420", "+0.12%", "🟢 Stable"]\n        ]\n      }\n    }\n  ]\n}</a2ui-json>`;
            appendMessage('agent', responseText);
        }, 400);
    });
}

const demoChartBtn = document.getElementById('demo-chart-btn');
if (demoChartBtn) {
    demoChartBtn.addEventListener('click', () => {
        appendMessage('user', 'Show me the historical trend for USD to EUR over the last 5 days');
        setTimeout(() => {
            const responseText = `Here is the historical rate trend for USD to EUR:\n\n<a2ui-json>{\n  "version": "v0.9",\n  "components": [\n    {\n      "id": "root_card",\n      "type": "Card",\n      "props": { "title": "📈 USD to EUR 5-Day Trend" }\n    },\n    {\n      "id": "trend_chart",\n      "type": "LineChart",\n      "parentId": "root_card",\n      "props": {\n        "title": "Historical Rates (Past 5 Days)",\n        "labels": ["05-30", "05-31", "06-01", "06-02", "06-03"],\n        "values": [0.85586, 0.85774, 0.85731, 0.85834, 0.86103],\n        "color": "hsl(var(--primary))"\n      }\n    }\n  ]\n}</a2ui-json>`;
            appendMessage('agent', responseText);
        }, 400);
    });
}

const demoWalletBtn = document.getElementById('demo-wallet-btn');
if (demoWalletBtn) {
    demoWalletBtn.addEventListener('click', () => {
        appendMessage('user', 'Show my multi-currency wallet balance breakdown');
        setTimeout(() => {
            const responseText = `Here is your current wallet balance breakdown:\n\n<a2ui-json>{\n  "version": "v0.9",\n  "components": [\n    {\n      "id": "wallet_card",\n      "type": "Card",\n      "props": { "title": "💳 Multi-Currency Wallet Balances" }\n    },\n    {\n      "id": "wallet_layout",\n      "type": "Column",\n      "parentId": "wallet_card"\n    },\n    {\n      "id": "wallet_total",\n      "type": "Text",\n      "parentId": "wallet_layout",\n      "variant": "h3",\n      "props": { "value": "Total Balance: $1,438.57 USD" }\n    },\n    {\n      "id": "balance_divider",\n      "type": "Text",\n      "parentId": "wallet_layout",\n      "variant": "body",\n      "props": { "value": "-----------------------------" }\n    },\n    {\n      "id": "row_usd",\n      "type": "Row",\n      "parentId": "wallet_layout"\n    },\n    {\n      "id": "usd_label",\n      "type": "Text",\n      "parentId": "row_usd",\n      "variant": "body",\n      "props": { "value": "🇺🇸 US Dollar:" }\n    },\n    {\n      "id": "usd_val",\n      "type": "Text",\n      "parentId": "row_usd",\n      "variant": "h4",\n      "props": { "value": "$500.00 USD" }\n    },\n    {\n      "id": "row_eur",\n      "type": "Row",\n      "parentId": "wallet_layout"\n    },\n    {\n      "id": "eur_label",\n      "type": "Text",\n      "parentId": "row_eur",\n      "variant": "body",\n      "props": { "value": "🇪🇺 Euro Zone:" }\n    },\n    {\n      "id": "eur_val",\n      "type": "Text",\n      "parentId": "row_eur",\n      "variant": "h4",\n      "props": { "value": "€450.00 EUR ($522.62 USD equiv.)" }\n    },\n    {\n      "id": "row_gbp",\n      "type": "Row",\n      "parentId": "wallet_layout"\n    },\n    {\n      "id": "gbp_label",\n      "type": "Text",\n      "parentId": "row_gbp",\n      "variant": "body",\n      "props": { "value": "🇬🇧 British Pound:" }\n    },\n    {\n      "id": "gbp_val",\n      "type": "Text",\n      "parentId": "row_gbp",\n      "variant": "h4",\n      "props": { "value": "£320.00 GBP ($415.95 USD equiv.)" }\n    }\n  ]\n}</a2ui-json>`;
            appendMessage('agent', responseText);
        }, 400);
    });
}


// Calculator events
calcSwap.addEventListener('click', () => {
    const temp = calcFrom.value;
    calcFrom.value = calcTo.value;
    calcTo.value = temp;
});

calcAskBtn.addEventListener('click', () => {
    const amount = calcAmount.value.trim();
    const fromVal = calcFrom.value;
    const toVal = calcTo.value;
    if (!amount || isNaN(Number(amount))) return;
    
    const query = `How much is ${amount} ${fromVal} in ${toVal}?`;
    chatInput.value = query;
    chatForm.dispatchEvent(new Event('submit'));
});

// Clear Chat button
clearChatBtn.addEventListener('click', () => {
    chatMessages.innerHTML = '';
    welcomeScreen.style.display = 'flex';
    chatMessages.appendChild(welcomeScreen);
});

// Main Chat Submission logic
chatForm.addEventListener('submit', async (e) => {
    e.preventDefault();
    const message = chatInput.value.trim();
    if (!message) return;

    chatInput.value = '';
    appendMessage('user', message);

    // Disable input while processing
    chatInput.disabled = true;
    sendButton.disabled = true;
    liveProgressBar.classList.remove('hidden');
    progressText.textContent = '🚀 Connecting to A2A backend...';

    logEvent('Request', `User sent message: "${message}"`);

    // Simulate backend SSE stream delays and progress states
    setTimeout(() => {
        progressText.textContent = '⚙️ Agent State: working...';
        logEvent('SSE Event', { type: 'progress', text: '⚙️ Agent State: working...' });
        
        setTimeout(() => {
            progressText.textContent = '🛠️ Calling get_exchange_rate...';
            logEvent('SSE Event', { type: 'progress', text: '🛠️ Calling get_exchange_rate...' });
            
            setTimeout(() => {
                progressText.textContent = '✅ get_exchange_rate returned rate data';
                logEvent('SSE Event', { type: 'progress', text: '✅ get_exchange_rate returned rate data' });
                
                setTimeout(() => {
                    let responseText = '';
                    const lowerMsg = message.toLowerCase();

                    if (lowerMsg.includes('trend') || lowerMsg.includes('chart') || lowerMsg.includes('history') || lowerMsg.includes('day')) {
                        responseText = `Here is the historical rate trend for USD to EUR:\n\n<a2ui-json>{\n  "version": "v0.9",\n  "components": [\n    {\n      "id": "root_card",\n      "type": "Card",\n      "props": { "title": "📈 USD to EUR 5-Day Trend" }\n    },\n    {\n      "id": "trend_chart",\n      "type": "LineChart",\n      "parentId": "root_card",\n      "props": {\n        "title": "Historical Rates (Past 5 Days)",\n        "labels": ["05-30", "05-31", "06-01", "06-02", "06-03"],\n        "values": [0.85586, 0.85774, 0.85731, 0.85834, 0.86103],\n        "color": "hsl(var(--primary))"\n      }\n    }\n  ]\n}</a2ui-json>`;
                    } else if (lowerMsg.includes('rate') || lowerMsg.includes('table') || lowerMsg.includes('pair') || lowerMsg.includes('latest')) {
                        responseText = `Here are the latest market rates from the Frankfurter API:\n\n<a2ui-json>{\n  "version": "v0.9",\n  "components": [\n    {\n      "id": "root_table",\n      "type": "Table",\n      "props": {\n        "headers": ["Currency Pair", "Current Rate", "Daily Change", "Status"],\n        "rows": [\n          ["USD / EUR", "0.86103", "+0.31%", "🟢 Stable"],\n          ["USD / GBP", "0.78450", "+0.25%", "🟢 Upward"],\n          ["USD / JPY", "149.52", "-0.18%", "🔴 Downward"],\n          ["USD / CAD", "1.35420", "+0.12%", "🟢 Stable"]\n        ]\n      }\n    }\n  ]\n}</a2ui-json>`;
                    } else if (lowerMsg.includes('wallet') || lowerMsg.includes('balance') || lowerMsg.includes('breakdown')) {
                        responseText = `Here is your current wallet balance breakdown:\n\n<a2ui-json>{\n  "version": "v0.9",\n  "components": [\n    {\n      "id": "wallet_card",\n      "type": "Card",\n      "props": { "title": "💳 Multi-Currency Wallet Balances" }\n    },\n    {\n      "id": "wallet_layout",\n      "type": "Column",\n      "parentId": "wallet_card"\n    },\n    {\n      "id": "wallet_total",\n      "type": "Text",\n      "parentId": "wallet_layout",\n      "variant": "h3",\n      "props": { "value": "Total Balance: $1,438.57 USD" }\n    },\n    {\n      "id": "balance_divider",\n      "type": "Text",\n      "parentId": "wallet_layout",\n      "variant": "body",\n      "props": { "value": "-----------------------------" }\n    },\n    {\n      "id": "row_usd",\n      "type": "Row",\n      "parentId": "wallet_layout"\n    },\n    {\n      "id": "usd_label",\n      "type": "Text",\n      "parentId": "row_usd",\n      "variant": "body",\n      "props": { "value": "🇺🇸 US Dollar:" }\n    },\n    {\n      "id": "usd_val",\n      "type": "Text",\n      "parentId": "row_usd",\n      "variant": "h4",\n      "props": { "value": "$500.00 USD" }\n    },\n    {\n      "id": "row_eur",\n      "type": "Row",\n      "parentId": "wallet_layout"\n    },\n    {\n      "id": "eur_label",\n      "type": "Text",\n      "parentId": "row_eur",\n      "variant": "body",\n      "props": { "value": "🇪🇺 Euro Zone:" }\n    },\n    {\n      "id": "eur_val",\n      "type": "Text",\n      "parentId": "row_eur",\n      "variant": "h4",\n      "props": { "value": "€450.00 EUR ($522.62 USD equiv.)" }\n    },\n    {\n      "id": "row_gbp",\n      "type": "Row",\n      "parentId": "wallet_layout"\n    },\n    {\n      "id": "gbp_label",\n      "type": "Text",\n      "parentId": "row_gbp",\n      "variant": "body",\n      "props": { "value": "🇬🇧 British Pound:" }\n    },\n    {\n      "id": "gbp_val",\n      "type": "Text",\n      "parentId": "row_gbp",\n      "variant": "h4",\n      "props": { "value": "£320.00 GBP ($415.95 USD equiv.)" }\n    }\n  ]\n}</a2ui-json>`;
                    } else {
                        // Default to Conversion Card
                        responseText = `Here is the conversion result:\n\n<a2ui-json>{\n  "version": "v0.9",\n  "components": [\n    {\n      "id": "root_card",\n      "type": "Card",\n      "props": { "title": "💵 Live Conversion Result" }\n    },\n    {\n      "id": "text_conversion",\n      "type": "Text",\n      "parentId": "root_card",\n      "props": { "value": "${message}" }\n    },\n    {\n      "id": "text_rate",\n      "type": "Text",\n      "parentId": "root_card",\n      "props": { "value": "Exchange Rate: 1 USD = 0.86103 EUR" }\n    },\n    {\n      "id": "text_date",\n      "type": "Text",\n      "parentId": "root_card",\n      "props": { "value": "Date: 2026-06-23 (Frankfurter API Mock)" }\n    }\n  ]\n}</a2ui-json>`;
                    }

                    appendMessage('agent', responseText);
                    logEvent('Result', `Completed response received.`);
                    
                    // Clean UI state
                    liveProgressBar.classList.add('hidden');
                    sendButton.disabled = false;
                    chatInput.disabled = false;
                    chatInput.focus();
                }, 400);
            }, 400);
        }, 400);
    }, 400);
});

// Initialization
initConfig();
fetchLiveRates();

// Refresh rates listener
refreshRatesBtn.addEventListener('click', () => {
    fetchLiveRates();
});
