import { useQuery } from '@tanstack/react-query'
import { fetchCalibrationMetrics } from '../api/calibration'

export function useCalibration(dateRange: string = 'all') {
  return useQuery({
    queryKey: ['calibration', dateRange],
    queryFn: () => fetchCalibrationMetrics(dateRange),
  })
}
