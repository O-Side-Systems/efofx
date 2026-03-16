import { apiClient } from './client'
import type { CalibrationMetrics, CalibrationTrendResponse } from '../types/calibration'

export async function fetchCalibrationMetrics(
  dateRange: string,
): Promise<CalibrationMetrics> {
  const { data } = await apiClient.get<CalibrationMetrics>(
    '/api/v1/calibration/metrics',
    { params: { date_range: dateRange } },
  )
  return data
}

export async function fetchCalibrationTrend(
  months: number = 12,
): Promise<CalibrationTrendResponse> {
  const { data } = await apiClient.get<CalibrationTrendResponse>(
    '/api/v1/calibration/trend',
    { params: { months } },
  )
  return data
}
