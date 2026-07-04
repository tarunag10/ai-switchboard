import { useState, useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import {
  Bar,
  BarChart,
  CartesianGrid,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts";
import {
  addDays,
  addMonths,
  buildHourlySavingsChartData,
  buildHourlySavingsWindow,
  buildMonthlySavingsChartData,
  buildMonthlySavingsWindow,
  compactNumber,
  currency,
  dayOfMonthTickFormatter,
  earliestHourlyDay,
  earliestSavingsMonth,
  formatMonthLabel,
  formatSelectedDayLabel,
  hourOfDayTickFormatter,
  startOfDay,
  startOfMonth,
} from "../lib/dashboardHelpers";
import { hasTauriEventRuntime } from "../lib/tauriRuntime";
import type { DailySavingsPoint, HourlySavingsPoint } from "../lib/types";
import { SavingsChartTooltip, type SavingsChartMode } from "./SavingsChartTooltip";

type SavingsChartView = "month" | "day";

export function DailySavingsChart({
  data,
  hourlyData,
  resetSignal,
  chartMode,
  setChartMode,
}: {
  data: DailySavingsPoint[];
  hourlyData: HourlySavingsPoint[];
  resetSignal: number;
  chartMode: SavingsChartMode;
  setChartMode: (mode: SavingsChartMode) => void;
}) {
  const currentMonth = startOfMonth(new Date());
  const today = startOfDay(new Date());
  const [visibleMonth, setVisibleMonth] = useState(() => currentMonth);
  const [visibleDay, setVisibleDay] = useState(() => today);
  const [view, setView] = useState<SavingsChartView>("day");
  const [savingsTodayUsd, setSavingsTodayUsd] = useState<number | null>(null);

  useEffect(() => {
    if (!hasTauriEventRuntime()) {
      return;
    }

    let unlisten: (() => void) | undefined;
    void listen<number>("savings-today-updated", (event) => {
      setSavingsTodayUsd(event.payload);
    }).then((fn) => {
      unlisten = fn;
    });
    return () => unlisten?.();
  }, []);
  const firstSavingsMonth = earliestSavingsMonth(data);
  const firstHourlyDay = earliestHourlyDay(hourlyData);
  const monthlyData = buildMonthlySavingsChartData(
    buildMonthlySavingsWindow(data, visibleMonth),
  );
  const hourlyChartData = buildHourlySavingsChartData(
    buildHourlySavingsWindow(hourlyData, visibleDay),
  );
  const chartData = view === "month" ? monthlyData : hourlyChartData;
  const canViewPreviousMonth = firstSavingsMonth
    ? visibleMonth > firstSavingsMonth
    : false;
  const canViewNextMonth = visibleMonth < currentMonth;
  const canViewPreviousDay = firstHourlyDay
    ? visibleDay > firstHourlyDay
    : false;
  const canViewNextDay = visibleDay < today;
  const label =
    view === "month"
      ? formatMonthLabel(visibleMonth)
      : formatSelectedDayLabel(visibleDay);

  useEffect(() => {
    const now = new Date();
    setVisibleMonth(startOfMonth(now));
    setVisibleDay(startOfDay(now));
  }, [resetSignal]);

  return (
    <div className="savings-chart">
      <section
        aria-label={
          view === "month"
            ? `Monthly history for ${label}`
            : `Hourly history for ${label}`
        }
        className="savings-chart__panel"
      >
        <div className="savings-chart__panel-header">
          <div className="savings-chart__title-row">
            <strong>History</strong>
            <div className="savings-chart__toggle" aria-label="Metric">
              <button
                className={`savings-chart__toggle-button${chartMode === "usd" ? " is-active" : ""}`}
                onClick={() => setChartMode("usd")}
                type="button"
              >
                $
              </button>
              <button
                className={`savings-chart__toggle-button${chartMode === "tokens" ? " is-active" : ""}`}
                onClick={() => setChartMode("tokens")}
                type="button"
              >
                tokens
              </button>
            </div>
          </div>
          <div className="savings-chart__nav">
            <div className="savings-chart__toggle" aria-label="History view">
              <button
                className={`savings-chart__toggle-button${view === "month" ? " is-active" : ""}`}
                onClick={() => setView("month")}
                type="button"
              >
                month
              </button>
              <button
                className={`savings-chart__toggle-button${view === "day" ? " is-active" : ""}`}
                onClick={() => setView("day")}
                type="button"
              >
                day
              </button>
            </div>
            <button
              className="savings-chart__nav-button"
              disabled={
                view === "month" ? !canViewPreviousMonth : !canViewPreviousDay
              }
              onClick={() =>
                view === "month"
                  ? setVisibleMonth((current) => addMonths(current, -1))
                  : setVisibleDay((current) => addDays(current, -1))
              }
              type="button"
            >
              Prev
            </button>
            <span className="savings-chart__range-label">{label}</span>
            <button
              className="savings-chart__nav-button"
              disabled={view === "month" ? !canViewNextMonth : !canViewNextDay}
              onClick={() =>
                view === "month"
                  ? setVisibleMonth((current) => addMonths(current, 1))
                  : setVisibleDay((current) => addDays(current, 1))
              }
              type="button"
            >
              Next
            </button>
          </div>
        </div>
        <div className="savings-chart__canvas savings-chart__canvas--combined">
          <div className="savings-chart__overlay" aria-hidden="true">
            <span className="savings-chart__overlay-total">
              {chartMode === "usd"
                ? currency(
                    Math.max(
                      0,
                      view === "day" &&
                        visibleDay >= today &&
                        savingsTodayUsd !== null
                        ? savingsTodayUsd
                        : chartData.reduce(
                            (s, d) => s + d.estimatedSavingsUsd,
                            0,
                          ),
                    ),
                  )
                : compactNumber(
                    Math.max(
                      0,
                      chartData.reduce((s, d) => s + d.estimatedTokensSaved, 0),
                    ),
                  )}
            </span>
            <span className="savings-chart__overlay-label">
              {view === "day" ? "saved today" : "saved this month"}
            </span>
          </div>
          <ResponsiveContainer height="100%" width="100%">
            <BarChart
              barCategoryGap="5%"
              barGap={1}
              data={chartData}
              margin={{ top: 64, right: 2, left: 2, bottom: 0 }}
            >
              <defs>
                <linearGradient
                  id="actualUsdGradient"
                  x1="0"
                  x2="0"
                  y1="0"
                  y2="1"
                >
                  <stop offset="0%" stopColor="#c96a30" />
                  <stop offset="100%" stopColor="#ED834E" />
                </linearGradient>
                <linearGradient
                  id="savingsUsdGradient"
                  x1="0"
                  x2="0"
                  y1="0"
                  y2="1"
                >
                  <stop offset="0%" stopColor="#3a7f74" />
                  <stop offset="100%" stopColor="#4F9E91" />
                </linearGradient>
                <linearGradient
                  id="actualTokensGradient"
                  x1="0"
                  x2="0"
                  y1="0"
                  y2="1"
                >
                  <stop offset="0%" stopColor="#c96a30" />
                  <stop offset="100%" stopColor="#ED834E" />
                </linearGradient>
                <linearGradient
                  id="savingsTokensGradient"
                  x1="0"
                  x2="0"
                  y1="0"
                  y2="1"
                >
                  <stop offset="0%" stopColor="#d4b832" stopOpacity="0.35" />
                  <stop offset="100%" stopColor="#EBCC6E" stopOpacity="0.25" />
                </linearGradient>
              </defs>
              <CartesianGrid
                stroke="rgba(36, 31, 29, 0.06)"
                strokeDasharray="2 8"
                vertical={false}
              />
              <XAxis
                axisLine={false}
                dataKey="bucketKey"
                interval={0}
                minTickGap={view === "month" ? 8 : 8}
                tickFormatter={
                  view === "month"
                    ? dayOfMonthTickFormatter
                    : hourOfDayTickFormatter
                }
                tick={{ fill: "#7a7169", fontSize: 10 }}
                tickLine={false}
              />
              <YAxis hide yAxisId="usd" />
              <YAxis hide yAxisId="tokens" />
              <Tooltip
                content={(props) => (
                  <SavingsChartTooltip {...props} chartMode={chartMode} />
                )}
                cursor={{ fill: "rgba(36, 31, 29, 0.05)" }}
              />
              {chartMode === "usd" && (
                <>
                  <Bar
                    dataKey="actualCostUsd"
                    fill="url(#actualUsdGradient)"
                    maxBarSize={16}
                    stackId="usd"
                    yAxisId="usd"
                  />
                  <Bar
                    dataKey="estimatedSavingsUsd"
                    fill="url(#savingsUsdGradient)"
                    maxBarSize={16}
                    radius={[1, 1, 0, 0]}
                    stackId="usd"
                    yAxisId="usd"
                  />
                </>
              )}
              {chartMode === "tokens" && (
                <>
                  <Bar
                    dataKey="totalTokensSent"
                    fill="url(#actualTokensGradient)"
                    maxBarSize={16}
                    stackId="tokens"
                    yAxisId="tokens"
                  />
                  <Bar
                    dataKey="estimatedTokensSaved"
                    fill="url(#savingsTokensGradient)"
                    maxBarSize={16}
                    stackId="tokens"
                    yAxisId="tokens"
                    shape={(props: any) => {
                      const { x, y, width, height, fill } = props;
                      if (!width || !height) return <g />;
                      const sw = 1.5;
                      return (
                        <rect
                          x={x + sw / 2}
                          y={y + sw / 2}
                          width={Math.max(0, width - sw)}
                          height={Math.max(0, height - sw)}
                          fill={fill}
                          stroke="#EBCC6E"
                          strokeWidth={sw}
                          rx={1}
                        />
                      );
                    }}
                  />
                </>
              )}
            </BarChart>
          </ResponsiveContainer>
        </div>
      </section>
    </div>
  );
}
