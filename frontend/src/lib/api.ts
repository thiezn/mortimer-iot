export type WeatherReading = {
  id: number
  temperature: number
  humidity: number
  recorded_at: string
}

export type WeatherHistoryResponse = {
  items: WeatherReading[]
  next_cursor: number | null
}

export type LatestWeatherResponse = {
  item: WeatherReading | null
}

export type HealthcheckResponse = {
  state: string
}

export type RangePreset = '24h' | '7d' | '30d' | 'all'

export function buildHistoryQuery(range: RangePreset, limit = 200): URLSearchParams {
  const params = new URLSearchParams()
  params.set('limit', String(limit))

  if (range !== 'all') {
    const now = Date.now()
    const offsetMs =
      range === '24h'
        ? 24 * 60 * 60 * 1000
        : range === '7d'
          ? 7 * 24 * 60 * 60 * 1000
          : 30 * 24 * 60 * 60 * 1000

    params.set('from', new Date(now - offsetMs).toISOString())
    params.set('to', new Date(now).toISOString())
  }

  return params
}

async function fetchJson<T>(path: string, signal?: AbortSignal): Promise<T> {
  const response = await fetch(path, {
    headers: {
      Accept: 'application/json',
    },
    signal,
  })

  if (!response.ok) {
    let message = `${response.status} ${response.statusText}`

    try {
      const error = (await response.json()) as { message?: string }
      if (error.message) {
        message = error.message
      }
    } catch {
      // Ignore non-JSON error bodies.
    }

    throw new Error(message)
  }

  return (await response.json()) as T
}

export async function fetchDashboardData(
  range: RangePreset,
  signal?: AbortSignal,
): Promise<{
  health: HealthcheckResponse
  latest: LatestWeatherResponse
  history: WeatherHistoryResponse
}> {
  const historyQuery = buildHistoryQuery(range)

  const [health, latest, history] = await Promise.all([
    fetchJson<HealthcheckResponse>('/api/v1/health', signal),
    fetchJson<LatestWeatherResponse>('/api/v1/weather/latest', signal),
    fetchJson<WeatherHistoryResponse>(`/api/v1/weather?${historyQuery.toString()}`, signal),
  ])

  return { health, latest, history }
}