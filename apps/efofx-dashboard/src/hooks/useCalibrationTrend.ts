import { useQuery } from '@tanstack/react-query'
import { fetchCalibrationTrend } from '../api/calibration'

export function useCalibrationTrend(months: number = 12) {
  return useQuery({
    queryKey: ['calibrationTrend', months],
    queryFn: () => fetchCalibrationTrend(months),
  })
}
