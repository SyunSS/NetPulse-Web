import http from './index'

export interface TaskPlan {
  id: string
  user_id: string
  name: string
  description: string | null
  cron_expression: string | null
  enabled: number
  last_run_at: string | null
  next_run_at: string | null
  created_at: string
  updated_at: string
}

export interface TaskPlanItem {
  id: string
  plan_id: string
  task_type: string
  urls: string        // JSON 数组
  options: string | null
  order_index: number
  created_at: string
}

export interface PlanItemInput {
  task_type: string
  urls: string[]
  options?: Record<string, unknown>
}

export interface PlanWithItems {
  id: string
  user_id: string
  name: string
  description: string | null
  cron_expression: string | null
  enabled: number
  last_run_at: string | null
  next_run_at: string | null
  created_at: string
  updated_at: string
  items: TaskPlanItem[]
}

export interface CreatePlanRequest {
  name: string
  description?: string
  cron_expression?: string
  enabled: boolean
  items: PlanItemInput[]
}

export interface PlanListResponse {
  plans: PlanWithItems[]
  total: number
  page: number
  size: number
}

export interface TaskPlanRun {
  id: string
  plan_id: string
  task_id: string | null
  triggered_by: string
  started_at: string
  finished_at: string | null
  status: string
  created_at: string
}

export interface RunPlanResponse {
  plan_run_id: string
  task_ids: string[]
}

export const planApi = {
  list(page = 1, size = 20) {
    return http.get<unknown, { code: number; msg: string; data: PlanListResponse }>(
      '/plan/list',
      { params: { page, size } },
    )
  },

  get(planId: string) {
    return http.get<unknown, { code: number; msg: string; data: PlanWithItems }>(
      `/plan/${planId}`,
    )
  },

  create(data: CreatePlanRequest) {
    return http.post<unknown, { code: number; msg: string; data: PlanWithItems }>(
      '/plan/create',
      data,
    )
  },

  update(planId: string, data: CreatePlanRequest) {
    return http.post<unknown, { code: number; msg: string; data: PlanWithItems }>(
      `/plan/${planId}/update`,
      data,
    )
  },

  delete(planId: string) {
    return http.post<unknown, { code: number; msg: string; data: null }>(
      `/plan/${planId}/delete`,
    )
  },

  run(planId: string) {
    return http.post<unknown, { code: number; msg: string; data: RunPlanResponse }>(
      `/plan/${planId}/run`,
    )
  },

  runs(planId: string, limit = 20) {
    return http.get<unknown, { code: number; msg: string; data: TaskPlanRun[] }>(
      `/plan/${planId}/runs`,
      { params: { limit } },
    )
  },
}
