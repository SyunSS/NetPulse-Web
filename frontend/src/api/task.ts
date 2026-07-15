import http from './index'

export interface CreateTaskRequest {
  task_type: string
  urls: string[]
  options?: Record<string, unknown>
}

export interface CreateTaskResponse {
  task_id: string
  status: string
}

export interface TestTask {
  id: string
  user_id: string
  task_type: string
  status: string
  config: string
  progress: number | null
  result: string | null
  error_msg: string | null
  created_at: string
  started_at: string | null
  finished_at: string | null
}

export interface WebsiteResult {
  id: string
  task_id: string
  url: string
  dns_time_ms: number | null
  dns_success: number | null
  tcp_time_ms: number | null
  tls_time_ms: number | null
  http_status: number | null
  ttfb_ms: number | null
  fp_ms: number | null
  fcp_ms: number | null
  dom_content_loaded_ms: number | null
  load_event_ms: number | null
  page_open_time_ms: number | null
  first_paint_ms: number | null
  resource_count: number | null
  resource_total_size: number | null
  final_url: string | null
  page_title: string | null
  screenshot_path: string | null
  error_msg: string | null
  created_at: string
}

export interface VideoResult {
  id: string
  task_id: string
  url: string
  platform: string | null
  first_play_time_ms: number | null
  buffer_count: number | null
  total_buffer_time_ms: number | null
  play_success: number | null
  video_download_speed: number | null
  video_size: number | null
  video_duration_ms: number | null
  dropped_frames: number | null
  decoded_frames: number | null
  screenshot_path: string | null
  page_title: string | null
  error_msg: string | null
  created_at: string
}

export interface DownloadResult {
  id: string
  task_id: string
  url: string
  download_speed: number | null
  avg_speed: number | null
  peak_speed: number | null
  download_time_ms: number | null
  file_size: number | null
  success: number | null
  error_msg: string | null
  created_at: string
}

export interface TaskListResponse {
  tasks: TestTask[]
  total: number
  page: number
  size: number
}

export interface DashboardStats {
  today_tests: number
  success_rate: number
  avg_dns: number
  avg_ttfb: number
  avg_page_time: number
  recent_tasks: TestTask[]
  trend_data: TrendPoint[]
}

export interface TrendPoint {
  time: string
  dns_ms: number
  ttfb_ms: number
  page_ms: number
}

export const taskApi = {
  create(data: CreateTaskRequest) {
    return http.post<unknown, { code: number; msg: string; data: CreateTaskResponse }>(
      '/task/create',
      data,
    )
  },

  list(page = 1, size = 20) {
    return http.get<unknown, { code: number; msg: string; data: TaskListResponse }>(
      '/task/list',
      { params: { page, size } },
    )
  },

  get(taskId: string) {
    return http.get<unknown, { code: number; msg: string; data: TestTask }>(
      `/task/${taskId}`,
    )
  },

  getResults(taskId: string) {
    return http.get<unknown, { code: number; msg: string; data: WebsiteResult[] }>(
      `/task/${taskId}/result`,
    )
  },

  getVideoResults(taskId: string) {
    return http.get<unknown, { code: number; msg: string; data: VideoResult[] }>(
      `/task/${taskId}/video-result`,
    )
  },

  getDownloadResults(taskId: string) {
    return http.get<unknown, { code: number; msg: string; data: DownloadResult[] }>(
      `/task/${taskId}/download-result`,
    )
  },

  cancel(taskId: string) {
    return http.post<unknown, { code: number; msg: string; data: null }>(
      `/task/${taskId}/cancel`,
    )
  },

  retry(taskId: string) {
    return http.post<unknown, { code: number; msg: string; data: CreateTaskResponse }>(
      `/task/${taskId}/retry`,
    )
  },
}

export const dashboardApi = {
  getStats() {
    return http.get<unknown, { code: number; msg: string; data: DashboardStats }>(
      '/dashboard/stats',
    )
  },
}
