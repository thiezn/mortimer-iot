<script lang="ts">
  import { onMount } from "svelte";

  import {
    fetchDashboardData,
    type RangePreset,
    type WeatherReading,
  } from "./lib/api";

  const ranges: RangePreset[] = ["24h", "7d", "30d", "all"];
  const POLL_INTERVAL_MS = 15_000;

  let selectedRange: RangePreset = "24h";
  let readings: WeatherReading[] = [];
  let latest: WeatherReading | null = null;
  let healthState = "UNKNOWN";
  let loading = true;
  let refreshing = false;
  let errorMessage = "";
  let lastUpdatedAt: Date | null = null;

  let intervalId: ReturnType<typeof setInterval> | null = null;
  let activeController: AbortController | null = null;

  function formatTimestamp(value: string): string {
    return new Intl.DateTimeFormat(undefined, {
      dateStyle: "medium",
      timeStyle: "short",
    }).format(new Date(value));
  }

  function formatNumber(value: number, suffix: string): string {
    return `${value.toFixed(1)}${suffix}`;
  }

  function createPath(values: number[]): string {
    if (values.length === 0) {
      return "";
    }

    const width = 100;
    const height = 100;
    const min = Math.min(...values);
    const max = Math.max(...values);
    const span = Math.max(max - min, 0.1);

    return values
      .map((value, index) => {
        const x =
          values.length === 1
            ? width / 2
            : (index / (values.length - 1)) * width;
        const y = height - ((value - min) / span) * height;
        return `${index === 0 ? "M" : "L"} ${x.toFixed(2)} ${y.toFixed(2)}`;
      })
      .join(" ");
  }

  function chartValues(
    key: "temperature" | "humidity" | "wind_speed",
  ): number[] {
    return [...readings].reverse().map((reading) => reading[key]);
  }

  async function loadDashboard(
    range = selectedRange,
    initial = false,
  ): Promise<void> {
    activeController?.abort();
    const controller = new AbortController();
    activeController = controller;

    if (initial) {
      loading = true;
    } else {
      refreshing = true;
    }

    errorMessage = "";

    try {
      const data = await fetchDashboardData(range, controller.signal);
      if (controller.signal.aborted) {
        return;
      }

      selectedRange = range;
      healthState = data.health.state;
      latest = data.latest.item;
      readings = data.history.items;
      lastUpdatedAt = new Date();
    } catch (error) {
      if (controller.signal.aborted) {
        return;
      }

      errorMessage =
        error instanceof Error ? error.message : "Unknown dashboard error";
    } finally {
      if (activeController === controller) {
        activeController = null;
      }
      loading = false;
      refreshing = false;
    }
  }

  function startPolling(): void {
    stopPolling();
    intervalId = setInterval(() => {
      if (document.visibilityState === "visible" && !refreshing) {
        void loadDashboard(selectedRange);
      }
    }, POLL_INTERVAL_MS);
  }

  function stopPolling(): void {
    if (intervalId) {
      clearInterval(intervalId);
      intervalId = null;
    }
  }

  function handleVisibilityChange(): void {
    if (document.visibilityState === "visible") {
      void loadDashboard(selectedRange);
      startPolling();
    } else {
      stopPolling();
    }
  }

  onMount(() => {
    void loadDashboard(selectedRange, true);
    startPolling();
    document.addEventListener("visibilitychange", handleVisibilityChange);

    return () => {
      activeController?.abort();
      stopPolling();
      document.removeEventListener("visibilitychange", handleVisibilityChange);
    };
  });
</script>

<svelte:head>
  <title>Mortimer IoT Weather</title>
  <meta
    name="description"
    content="Weather dashboard for Mortimer IoT measurements from the Nano 33 IoT device."
  />
</svelte:head>

<main class="page-shell">
  <section class="hero-panel">
    <div>
      <p class="eyebrow">Mortimer IoT</p>
      <h1>Weather station telemetry</h1>
      <p class="hero-copy">
        Live temperature, humidity and wind speed readings
      </p>
    </div>

    <div class="status-card" data-ok={healthState === "OK"}>
      <span class="status-label">Service status</span>
      <strong>{healthState}</strong>
      <span
        >{lastUpdatedAt
          ? `Updated ${lastUpdatedAt.toLocaleTimeString()}`
          : "Waiting for first refresh"}</span
      >
    </div>
  </section>

  <section class="toolbar">
    <div class="range-group" aria-label="History range">
      {#each ranges as range}
        <button
          class:selected={range === selectedRange}
          type="button"
          on:click={() => void loadDashboard(range, true)}
        >
          {range}
        </button>
      {/each}
    </div>

    <button
      class="refresh-button"
      type="button"
      on:click={() => void loadDashboard(selectedRange)}
    >
      {refreshing ? "Refreshing…" : "Refresh now"}
    </button>
  </section>

  {#if errorMessage}
    <section class="alert-panel" role="alert">
      <strong>Dashboard request failed.</strong>
      <span>{errorMessage}</span>
    </section>
  {/if}

  <section class="metric-grid">
    <article class="metric-card accent-red">
      <span>Latest temperature</span>
      <strong
        >{latest ? formatNumber(latest.temperature, "°C") : "No data"}</strong
      >
      <small
        >{latest
          ? formatTimestamp(latest.recorded_at)
          : "Waiting for first reading"}</small
      >
    </article>

    <article class="metric-card accent-blue">
      <span>Latest humidity</span>
      <strong>{latest ? formatNumber(latest.humidity, "%") : "No data"}</strong>
      <small
        >{latest
          ? formatTimestamp(latest.recorded_at)
          : "Waiting for first reading"}</small
      >
    </article>

    <article class="metric-card accent-green">
      <span>Latest wind speed</span>
      <strong
        >{latest ? formatNumber(latest.wind_speed, " km/h") : "No data"}</strong
      >
      <small
        >{latest
          ? formatTimestamp(latest.recorded_at)
          : "Waiting for first reading"}</small
      >
    </article>
  </section>

  <section class="chart-grid">
    <article class="chart-card">
      <div class="chart-header">
        <h2>Temperature trend</h2>
        <span>{readings.length} points</span>
      </div>

      {#if loading}
        <div class="chart-placeholder">Loading temperature history…</div>
      {:else if readings.length === 0}
        <div class="chart-placeholder">
          No temperature readings in the selected range.
        </div>
      {:else}
        <svg
          viewBox="0 0 100 100"
          preserveAspectRatio="none"
          aria-label="Temperature chart"
        >
          <path d={createPath(chartValues("temperature"))} />
        </svg>
      {/if}
    </article>

    <article class="chart-card">
      <div class="chart-header">
        <h2>Humidity trend</h2>
        <span>{readings.length} points</span>
      </div>

      {#if loading}
        <div class="chart-placeholder">Loading humidity history…</div>
      {:else if readings.length === 0}
        <div class="chart-placeholder">
          No humidity readings in the selected range.
        </div>
      {:else}
        <svg
          viewBox="0 0 100 100"
          preserveAspectRatio="none"
          aria-label="Humidity chart"
        >
          <path d={createPath(chartValues("humidity"))} />
        </svg>
      {/if}
    </article>

    <article class="chart-card">
      <div class="chart-header">
        <h2>Wind speed trend</h2>
        <span>{readings.length} points</span>
      </div>

      {#if loading}
        <div class="chart-placeholder">Loading wind speed history…</div>
      {:else if readings.length === 0}
        <div class="chart-placeholder">
          No wind speed readings in the selected range.
        </div>
      {:else}
        <svg
          viewBox="0 0 100 100"
          preserveAspectRatio="none"
          aria-label="Wind speed chart"
        >
          <path d={createPath(chartValues("wind_speed"))} />
        </svg>
      {/if}
    </article>
  </section>

  <section class="table-card">
    <div class="chart-header">
      <h2>Recent readings</h2>
      <span
        >{readings.length === 0 ? "Empty" : `${readings.length} loaded`}</span
      >
    </div>

    {#if loading}
      <div class="table-placeholder">Loading recent readings…</div>
    {:else if readings.length === 0}
      <div class="table-placeholder">No readings have been stored yet.</div>
    {:else}
      <div class="table-scroll">
        <table>
          <thead>
            <tr>
              <th>ID</th>
              <th>Recorded at</th>
              <th>Temperature</th>
              <th>Humidity</th>
              <th>Wind speed</th>
            </tr>
          </thead>
          <tbody>
            {#each readings as reading}
              <tr>
                <td>{reading.id}</td>
                <td>{formatTimestamp(reading.recorded_at)}</td>
                <td>{formatNumber(reading.temperature, "°C")}</td>
                <td>{formatNumber(reading.humidity, "%")}</td>
                <td>{formatNumber(reading.wind_speed, " km/h")}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {/if}
  </section>
</main>
