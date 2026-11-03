/**
 * Thin wrapper around ECharts for cumulative-result curves.
 * Colours: categorical slots 1–3 of the dataviz reference palette, validated
 * against our surfaces (light #ffffff, dark #171a21) with validate_palette.js.
 * Slot 3 is under 3:1 on light, so every series is labelled with its final
 * value in the legend (relief rule) — in the legend rather than at the line
 * ends, where close values made labels collide.
 */
import { useEffect, useRef } from "react";
import type { EChartsType } from "echarts/core";

const SERIES = {
  light: ["#2a78d6", "#eb6834", "#1baf7a"],
  dark: ["#3987e5", "#d95926", "#199e70"],
};
const CHROME = {
  light: { ink: "#52514e", muted: "#898781", grid: "#e1e0d9", axis: "#c3c2b7", surface: "#ffffff" },
  dark: { ink: "#c3c2b7", muted: "#898781", grid: "#2c2c2a", axis: "#383835", surface: "#171a21" },
};

export interface Line {
  name: string;
  data: number[];
}

export function LineChart({
  x,
  lines,
  xLabel,
  format,
  testId,
}: {
  x: number[];
  lines: Line[];
  xLabel: string;
  format: (v: number) => string;
  testId?: string;
}) {
  const el = useRef<HTMLDivElement>(null);
  const chart = useRef<EChartsType | null>(null);

  useEffect(() => {
    let disposed = false;
    let ro: ResizeObserver | null = null;
    void (async () => {
      const echarts = await import("echarts/core");
      const { LineChart: L } = await import("echarts/charts");
      const { GridComponent, TooltipComponent, LegendComponent } = await import("echarts/components");
      const { CanvasRenderer } = await import("echarts/renderers");
      echarts.use([L, GridComponent, TooltipComponent, LegendComponent, CanvasRenderer]);
      if (disposed || !el.current) return;
      const mode = document.documentElement.dataset.theme === "light" ? "light" : "dark";
      const c = CHROME[mode];
      chart.current?.dispose();
      chart.current = echarts.init(el.current, undefined, { renderer: "canvas" });
      chart.current.setOption({
        animation: false,
        color: SERIES[mode],
        grid: { left: 64, right: 24, top: 40, bottom: 40 },
        legend: {
          top: 0,
          left: 0,
          textStyle: { color: c.ink },
          icon: "roundRect",
          itemWidth: 14,
          itemHeight: 4,
          formatter: (name: string) => {
            const l = lines.find((x) => x.name === name);
            const end = l?.data[l.data.length - 1];
            return end === undefined ? name : `${name}  ${format(end)}`;
          },
        },
        tooltip: {
          trigger: "axis",
          axisPointer: { type: "line", lineStyle: { color: c.muted } },
          backgroundColor: c.surface,
          borderColor: c.axis,
          textStyle: { color: c.ink },
          valueFormatter: (v: unknown) => format(Number(v)),
        },
        xAxis: {
          type: "category",
          data: x,
          name: xLabel,
          nameLocation: "middle",
          nameGap: 26,
          nameTextStyle: { color: c.muted },
          axisLine: { lineStyle: { color: c.axis } },
          axisTick: { show: false },
          axisLabel: { color: c.muted, hideOverlap: true },
        },
        yAxis: {
          type: "value",
          axisLine: { show: false },
          splitLine: { lineStyle: { color: c.grid } },
          axisLabel: { color: c.muted, formatter: (v: number) => format(v) },
        },
        series: lines.map((l) => ({
          name: l.name,
          type: "line",
          data: l.data,
          showSymbol: false,
          symbolSize: 8,
          lineStyle: { width: 2 },
          emphasis: { focus: "series" },
        })),
      });
      ro = new ResizeObserver(() => chart.current?.resize());
      ro.observe(el.current);
    })();
    return () => {
      disposed = true;
      ro?.disconnect();
      chart.current?.dispose();
      chart.current = null;
    };
  }, [x, lines, xLabel, format]);

  return <div ref={el} className="h-80 w-full" data-testid={testId} role="img" aria-label={lines.map((l) => l.name).join(", ")} />;
}
